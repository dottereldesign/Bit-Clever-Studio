use crate::{content, db, file_moves, managed_library};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

fn fixture() -> (tempfile::TempDir, PathBuf, PathBuf) {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap();
    let library = root.join("library");
    fs::create_dir_all(library.join("Library Data")).unwrap();
    let path = library.join("Library Data/Library.sqlite");
    db::init(&path).unwrap();
    let image = image::RgbImage::from_pixel(2, 2, image::Rgb([12, 34, 56]));
    let c = db::connect(&path).unwrap();
    db::set_setting(&c, "managed_root", library.to_str().unwrap()).unwrap();
    c.execute_batch("INSERT INTO projects VALUES(1,'Project');")
        .unwrap();
    for id in [1, 2] {
        let file = library.join(format!("{id}.png"));
        image.save(&file).unwrap();
        c.execute(
            "INSERT INTO items(id,path,name,kind,added) VALUES(?,?,?,'image',1)",
            rusqlite::params![id, file.to_str().unwrap(), format!("Image {id}")],
        )
        .unwrap();
    }
    c.execute_batch("INSERT INTO item_projects VALUES(2,1,1);")
        .unwrap();
    (temp, path, library)
}
fn pending(path: &Path) -> i64 {
    db::connect(path)
        .unwrap()
        .query_row("SELECT count(*) FROM file_move_journal", [], |r| r.get(0))
        .unwrap()
}
fn archived(path: &Path) -> i64 {
    db::connect(path)
        .unwrap()
        .query_row("SELECT archived FROM items WHERE id=2", [], |r| r.get(0))
        .unwrap()
}
fn crash(path: &Path, root: &Path, operation: &str, point: &str) {
    crash_using(path, root, operation, point, false);
}
fn crash_using(path: &Path, root: &Path, operation: &str, point: &str, copy: bool) {
    crash_ids(path, root, operation, point, copy, "2");
}
fn crash_ids(path: &Path, root: &Path, operation: &str, point: &str, copy: bool, ids: &str) {
    let output = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "interruption_tests::crash_worker",
            "--ignored",
            "--nocapture",
        ])
        .env("DESIGN_TEST_IDS", ids)
        .env("DESIGN_TEST_FORCE_COPY", if copy { "1" } else { "0" })
        .env("DESIGN_TEST_CRASH_POINT", point)
        .env("DESIGN_TEST_DB", path)
        .env("DESIGN_TEST_ROOT", root)
        .env("DESIGN_TEST_OPERATION", operation)
        .output()
        .unwrap();
    assert_eq!(
        output.status.code(),
        Some(86),
        "Worker did not reach {point}: {}",
        String::from_utf8_lossy(&output.stdout)
    );
}
#[test]
#[ignore = "Child process for synthetic crash injection only"]
fn crash_worker() {
    let path = PathBuf::from(std::env::var_os("DESIGN_TEST_DB").expect("Parent fixture required"));
    let root = PathBuf::from(std::env::var_os("DESIGN_TEST_ROOT").unwrap());
    match std::env::var("DESIGN_TEST_OPERATION").unwrap().as_str() {
        "remove" => {
            content::remove_duplicates(
                &path,
                &root,
                1,
                std::env::var("DESIGN_TEST_IDS")
                    .unwrap()
                    .split(',')
                    .map(|id| id.parse().unwrap())
                    .collect(),
            )
            .unwrap();
        }
        "restore" => content::restore(&path, 2).unwrap(),
        _ => panic!("Unknown fixture operation"),
    }
    panic!("Crash checkpoint not reached");
}
#[test]
fn interrupted_duplicate_removal_rolls_back_or_keeps_the_committed_result() {
    for point in [
        "remove.after_intent",
        "move.after_publish",
        "remove.after_move",
        "remove.after_commit",
    ] {
        let (_temp, path, root) = fixture();
        let bytes = fs::read(root.join("2.png")).unwrap();
        crash(&path, &root, "remove", point);
        file_moves::reconcile(&path).unwrap();
        file_moves::reconcile(&path).unwrap();
        assert_eq!(pending(&path), 0);
        let committed = point == "remove.after_commit";
        assert_eq!(archived(&path), i64::from(committed));
        assert_eq!(root.join("2.png").exists(), !committed);
        let library = db::load(&path, false).unwrap();
        assert_eq!(
            library.items[0].projects,
            if committed { vec![1] } else { vec![] }
        );
        if !committed {
            assert_eq!(fs::read(root.join("2.png")).unwrap(), bytes);
            content::remove_duplicates(&path, &root, 1, vec![2]).unwrap();
        }
        content::restore(&path, 2).unwrap();
        assert_eq!(fs::read(root.join("2.png")).unwrap(), bytes);
    }
}
#[test]
fn interrupted_duplicate_restore_can_be_retried_without_losing_the_recovery_copy() {
    for point in [
        "restore.after_intent",
        "move.after_publish",
        "restore.after_move",
        "restore.after_commit",
    ] {
        let (_temp, path, root) = fixture();
        let bytes = fs::read(root.join("2.png")).unwrap();
        content::remove_duplicates(&path, &root, 1, vec![2]).unwrap();
        crash(&path, &root, "restore", point);
        file_moves::reconcile(&path).unwrap();
        file_moves::reconcile(&path).unwrap();
        assert_eq!(pending(&path), 0);
        if point != "restore.after_commit" {
            assert_eq!(archived(&path), 1);
            assert!(!root.join("2.png").exists());
            assert_eq!(content::recovery(&path).unwrap().len(), 1);
            content::restore(&path, 2).unwrap();
        }
        content::restore(&path, 2).unwrap(); // Safe retry even after a committed restore.
        assert_eq!(archived(&path), 0);
        assert_eq!(fs::read(root.join("2.png")).unwrap(), bytes);
        assert!(content::recovery(&path).unwrap().is_empty());
    }
}
#[test]
fn move_commit_failure_rolls_back_the_file_and_memberships_then_allows_retry() {
    let (_temp, path, root) = fixture();
    let c = db::connect(&path).unwrap();
    c.execute_batch("CREATE TABLE commit_failure(value INTEGER REFERENCES projects(id) DEFERRABLE INITIALLY DEFERRED);
        CREATE TRIGGER fail_archive AFTER UPDATE OF archived ON items BEGIN INSERT INTO commit_failure VALUES(999); END;").unwrap();
    assert!(content::remove_duplicates(&path, &root, 1, vec![2]).is_err());
    assert_eq!(archived(&path), 0);
    assert!(root.join("2.png").is_file());
    assert_eq!(pending(&path), 0);
    assert!(db::load(&path, false).unwrap().items[0].projects.is_empty());
    assert!(content::recovery(&path).unwrap().is_empty());
    c.execute_batch("DROP TRIGGER fail_archive;").unwrap();
    content::remove_duplicates(&path, &root, 1, vec![2]).unwrap();
    c.execute_batch("CREATE TRIGGER fail_restore AFTER UPDATE OF archived ON items BEGIN INSERT INTO commit_failure VALUES(999); END;").unwrap();
    assert!(content::restore(&path, 2).is_err());
    assert_eq!(archived(&path), 1);
    assert!(!root.join("2.png").exists());
    assert_eq!(pending(&path), 0);
    c.execute_batch("DROP TRIGGER fail_restore;").unwrap();
    content::restore(&path, 2).unwrap();
    assert!(root.join("2.png").is_file());
}
#[test]
fn interrupted_moves_survive_library_relocation_and_conflicts_remain_recoverable() {
    let (temp, path, root) = fixture();
    crash(&path, &root, "remove", "remove.after_move");
    let moved = temp.path().canonicalize().unwrap().join("moved");
    fs::rename(&root, &moved).unwrap();
    let path = moved.join("Library Data/Library.sqlite");
    managed_library::rebase(&path, &moved).unwrap();
    fs::write(moved.join("2.png"), b"new unrelated file").unwrap();
    assert!(file_moves::reconcile(&path).is_err());
    assert_eq!(pending(&path), 1);
    assert_eq!(
        fs::read(moved.join("2.png")).unwrap(),
        b"new unrelated file"
    );
    fs::remove_file(moved.join("2.png")).unwrap();
    assert_eq!(managed_library::open(&moved).unwrap(),path);
    assert_eq!(pending(&path), 0);
    content::remove_duplicates(&path, &moved, 1, vec![2]).unwrap();
    content::restore(&path, 2).unwrap();
    assert!(moved.join("2.png").is_file());
}
#[test]
fn unavailable_recovery_location_keeps_the_intent_until_it_returns() {
    let (_temp, path, root) = fixture();
    crash(&path, &root, "remove", "remove.after_move");
    let recovery = root.join("removed-duplicates");
    let offline = root.join("offline");
    fs::rename(&recovery, &offline).unwrap();
    assert!(file_moves::reconcile(&path)
        .unwrap_err()
        .contains("unavailable"));
    assert_eq!(pending(&path), 1);
    file_moves::recover_on_startup(&path).unwrap();
    assert!(db::load(&path, false)
        .unwrap()
        .maintenance_warning
        .unwrap()
        .contains("unavailable"));
    fs::rename(&offline, &recovery).unwrap();
    file_moves::reconcile(&path).unwrap();
    assert_eq!(pending(&path), 0);
    assert!(root.join("2.png").is_file());
    assert!(db::load(&path, false)
        .unwrap()
        .maintenance_warning
        .is_none());
}

#[test]
fn interrupted_copy_fallback_preserves_bytes_and_can_be_retried() {
    for operation in ["remove", "restore"] {
        for point in [
            "move.after_publish",
            if operation == "remove" {
                "remove.after_move"
            } else {
                "restore.after_move"
            },
        ] {
            let (_temp, path, root) = fixture();
            let bytes = fs::read(root.join("2.png")).unwrap();
            if operation == "restore" {
                content::remove_duplicates(&path, &root, 1, vec![2]).unwrap();
            }
            crash_using(&path, &root, operation, point, true);
            file_moves::reconcile(&path).unwrap();
            if operation == "remove" {
                assert_eq!(fs::read(root.join("2.png")).unwrap(), bytes);
                content::remove_duplicates(&path, &root, 1, vec![2]).unwrap();
            }
            content::restore(&path, 2).unwrap();
            assert_eq!(fs::read(root.join("2.png")).unwrap(), bytes);
            assert_eq!(pending(&path), 0);
        }
    }
}

#[test]
#[cfg(windows)]
fn interrupted_library_switch_preserves_pointer_and_unavailable_drive_is_retryable() {
    use std::os::windows::fs::OpenOptionsExt;
    let (_first, path1, root1) = fixture();
    let (_second, path2, root2) = fixture();
    managed_library::organise(&path1).unwrap();
    managed_library::organise(&path2).unwrap();
    let config = root1.join("config");
    fs::create_dir(&config).unwrap();
    managed_library::activate(&config, &root1).unwrap();
    let pointer = config.join("active-library.json");
    let before = fs::read(&pointer).unwrap();
    // A reader denying delete sharing simulates a failed atomic pointer replacement.
    let held = fs::OpenOptions::new()
        .read(true)
        .share_mode(1)
        .open(&pointer)
        .unwrap();
    assert!(managed_library::activate(&config, &root2).is_err());
    assert_eq!(fs::read(&pointer).unwrap(), before);
    drop(held);
    managed_library::activate(&config, &root2).unwrap();
    let selected = fs::read(&pointer).unwrap();
    let offline = root2.with_file_name("offline");
    fs::rename(&root2, &offline).unwrap();
    assert!(managed_library::selected_db(&config)
        .unwrap_err()
        .contains("unavailable"));
    assert_eq!(fs::read(&pointer).unwrap(), selected);
    fs::rename(&offline, &root2).unwrap();
    assert_eq!(managed_library::selected_db(&config).unwrap(), path2);
}

#[test]
fn partially_committed_duplicate_batch_can_be_retried_as_a_whole() {
    let (_temp, path, root) = fixture();
    let third = root.join("3.png");
    fs::copy(root.join("2.png"), &third).unwrap();
    db::connect(&path)
        .unwrap()
        .execute(
            "INSERT INTO items(id,path,name,kind,added) VALUES(3,?,'Third','image',1)",
            [third.to_str().unwrap()],
        )
        .unwrap();
    crash_ids(&path, &root, "remove", "remove.after_commit", false, "2,3");
    assert_eq!(content::recovery(&path).unwrap().len(), 1);
    assert_eq!(
        content::remove_duplicates(&path, &root, 1, vec![2, 3]).unwrap(),
        1
    );
    assert_eq!(
        content::remove_duplicates(&path, &root, 1, vec![2, 3]).unwrap(),
        0
    );
    assert_eq!(db::load(&path, false).unwrap().items.len(), 1);
    assert_eq!(content::recovery(&path).unwrap().len(), 2);
    for id in [2, 3] {
        content::restore(&path, id).unwrap();
        content::restore(&path, id).unwrap();
        assert!(root.join(format!("{id}.png")).is_file());
    }
}
