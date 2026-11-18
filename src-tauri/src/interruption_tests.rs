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
        "organise" => managed_library::organise(&path).unwrap(),
        "import" => db::import(&path, root.join("incoming.json")).unwrap(),
        "capture" => {content::capture(&path,capture_input(&root)).unwrap();},
        "download"|"download_cancel" => {
            let result=crate::download::import_cancelled(&path,&root,&root.parent().unwrap().join("download.mp4"),"Downloaded",&download_request(),&std::sync::atomic::AtomicBool::new(false));
            if std::env::var("DESIGN_TEST_OPERATION").unwrap()=="download_cancel" {
                let committed=std::env::var("DESIGN_TEST_CRASH_POINT").unwrap()=="download.after_commit";
                assert_eq!(result.is_ok(),committed);if !committed{assert_eq!(result.err().unwrap(),"Download cancelled.");}
                assert_eq!(import_pending(&path),0);assert_eq!(db::load(&path,false).unwrap().items.len(),if committed{3}else{2});std::process::exit(86);
            }
            result.unwrap();
        },
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

fn import_fixture() -> (tempfile::TempDir, PathBuf, PathBuf, PathBuf) {
    let (temp,path,root)=fixture();
    let original=temp.path().join("external.png");
    image::RgbImage::from_pixel(3,3,image::Rgb([98,76,54])).save(&original).unwrap();
    let metadata=root.join("incoming.json");db::export(&path,metadata.clone()).unwrap();
    let mut backup:serde_json::Value=serde_json::from_slice(&fs::read(&metadata).unwrap()).unwrap();
    let mut item=backup["library"]["items"][0].clone();item["path"]=serde_json::json!(original);item["name"]=serde_json::json!("External");item["thumbnail"]=serde_json::json!(original);
    backup["library"]["items"]=serde_json::json!([item]);
    fs::write(metadata,serde_json::to_vec(&backup).unwrap()).unwrap();
    (temp,path,root,original)
}
fn import_pending(path:&Path)->i64 {
    db::connect(path).unwrap().query_row("SELECT count(*) FROM import_copy_journal",[],|r|r.get(0)).unwrap()
}
#[test]
fn interrupted_organisation_restores_paths_or_preserves_the_commit_and_retries() {
    for point in ["organise.after_intent","move.after_publish","organise.after_move","organise.after_commit"] {
        let (_temp,path,root)=fixture();
        let hashes:Vec<_>=db::load(&path,false).unwrap().items.iter().map(|i|content::hash_file(Path::new(&i.path)).unwrap()).collect();
        crash(&path,&root,"organise",point);
        file_moves::reconcile(&path).unwrap();
        for (item,hash) in db::load(&path,false).unwrap().items.iter().zip(&hashes){assert_eq!(&content::hash_file(Path::new(&item.path)).unwrap(),hash);}
        managed_library::organise(&path).unwrap();managed_library::organise(&path).unwrap();
        assert_eq!(pending(&path),0);
        let library=db::load(&path,false).unwrap();assert_eq!(library.items.len(),2);
        assert!(library.items.iter().all(|i|Path::new(&i.path).parent()==Some(root.join("Design Inspo").as_path())));
        assert!(library.items.iter().find(|i|i.id==2).unwrap().projects.contains(&1));
    }
}
#[test]
fn organisation_commit_failure_rolls_files_back() {
    let (_temp,path,root)=fixture();
    db::connect(&path).unwrap().execute_batch("CREATE TRIGGER reject_path BEFORE UPDATE OF path ON items BEGIN SELECT RAISE(ABORT,'injected failure'); END;").unwrap();
    assert!(managed_library::organise(&path).is_err());assert_eq!(pending(&path),0);
    assert!(root.join("1.png").is_file());assert!(root.join("2.png").is_file());
    db::connect(&path).unwrap().execute_batch("DROP TRIGGER reject_path;").unwrap();
    managed_library::organise(&path).unwrap();
}
#[test]
fn interrupted_import_cleans_provisional_copies_and_retries_without_duplicates() {
    for point in ["import.after_copy_intent","import.after_publish","import.before_commit","import.after_commit"] {
        let (_temp,path,root,original)=import_fixture();let bytes=fs::read(&original).unwrap();
        crash(&path,&root,"import",point);
        assert_eq!(db::load(&path,false).unwrap().items.len(),if point=="import.after_commit"{3}else{2});
        file_moves::reconcile(&path).unwrap();assert_eq!(import_pending(&path),0);
        db::import(&path,root.join("incoming.json")).unwrap();db::import(&path,root.join("incoming.json")).unwrap();
        let library=db::load(&path,false).unwrap();assert_eq!(library.items.len(),3);
        let imported=library.items.iter().find(|i|i.name=="External").unwrap();assert_eq!(fs::read(&imported.path).unwrap(),bytes);assert_eq!(fs::read(&imported.thumbnail).unwrap(),bytes);
        assert_eq!(fs::read(&original).unwrap(),bytes);assert_eq!(import_pending(&path),0);
        assert_eq!(fs::read_dir(root.join("Design Inspo")).unwrap().count(),1);
    }
}
#[test]
fn invalid_import_rolls_back_its_published_copies() {
    let (_temp,path,root,original)=import_fixture();let metadata=root.join("incoming.json");
    let mut backup:serde_json::Value=serde_json::from_slice(&fs::read(&metadata).unwrap()).unwrap();
    backup["library"]["items"][0]["kind"]=serde_json::json!("invalid");fs::write(&metadata,serde_json::to_vec(&backup).unwrap()).unwrap();
    assert!(db::import(&path,metadata).is_err());assert_eq!(db::load(&path,false).unwrap().items.len(),2);
    assert_eq!(import_pending(&path),0);assert_eq!(fs::read_dir(root.join("Design Inspo")).unwrap().count(),0);assert!(original.is_file());
}
#[test]
fn interrupted_import_preserves_conflicts_and_rebases_with_the_library() {
    let (temp,path,root,original)=import_fixture();let bytes=fs::read(&original).unwrap();
    crash(&path,&root,"import","import.after_publish");
    let target:String=db::connect(&path).unwrap().query_row("SELECT destination FROM import_copy_journal",[],|r|r.get(0)).unwrap();
    fs::write(&target,b"Changed after interruption").unwrap();assert!(file_moves::reconcile(&path).is_err());
    assert_eq!(fs::read(&target).unwrap(),b"Changed after interruption");assert_eq!(import_pending(&path),1);
    fs::write(&target,&bytes).unwrap();let moved=temp.path().join("Relocated");fs::rename(&root,&moved).unwrap();
    let moved_db=moved.join("Library Data/Library.sqlite");managed_library::rebase(&moved_db,&moved).unwrap();
    file_moves::reconcile(&moved_db).unwrap();assert_eq!(import_pending(&moved_db),0);
    db::import(&moved_db,moved.join("incoming.json")).unwrap();assert_eq!(db::load(&moved_db,false).unwrap().items.len(),3);
    assert_eq!(fs::read(original).unwrap(),bytes);
}

fn capture_input(root:&Path)->content::Capture {
    serde_json::from_value(serde_json::json!({"source":"Design","projects":[1],"files":[{"path":root.parent().unwrap().join("external.png"),"name":"External.png"}]})).unwrap()
}
#[test]
fn interrupted_capture_recovers_media_memberships_and_history() {
    for point in ["capture.after_copy_intent","capture.after_publish","capture.before_commit","capture.after_commit"] {
        let (_temp,path,root,original)=import_fixture();let bytes=fs::read(&original).unwrap();
        let destination=root.join("Design Inspo");fs::create_dir_all(&destination).unwrap();
        db::set_setting(&db::connect(&path).unwrap(),"root",destination.to_str().unwrap()).unwrap();
        crash(&path,&root,"capture",point);file_moves::reconcile(&path).unwrap();
        content::capture(&path,capture_input(&root)).unwrap();content::capture(&path,capture_input(&root)).unwrap();
        let library=db::load(&path,false).unwrap();assert_eq!(library.items.len(),3);
        let item=library.items.iter().find(|i|i.name=="External").unwrap();assert!(item.projects.contains(&1));
        assert_eq!(fs::read(&item.path).unwrap(),bytes);assert_eq!(fs::read(&original).unwrap(),bytes);
        assert_eq!(import_pending(&path),0);assert_eq!(fs::read_dir(destination).unwrap().count(),1);
        assert_eq!(library.undo_entries.iter().filter(|e|e.label=="Add reference").count(),1);
    }
}

#[test]
fn failed_capture_keeps_original_and_removes_only_its_provisional_copy() {
    let (_temp,path,root,original)=import_fixture();let bytes=fs::read(&original).unwrap();
    let destination=root.join("Design Inspo");fs::create_dir_all(&destination).unwrap();
    db::set_setting(&db::connect(&path).unwrap(),"root",destination.to_str().unwrap()).unwrap();
    let mut input=capture_input(&root);input.projects=vec![9999];
    assert!(content::capture(&path,input).is_err());assert_eq!(import_pending(&path),0);
    assert_eq!(fs::read_dir(&destination).unwrap().count(),0);assert_eq!(fs::read(&original).unwrap(),bytes);
    assert_eq!(db::load(&path,false).unwrap().items.len(),2);
    content::capture(&path,capture_input(&root)).unwrap();assert_eq!(db::load(&path,false).unwrap().items.len(),3);
}

fn download_request()->crate::download::Request {crate::download::Request{url:"https://example.test/video".into(),projects:vec![1],tags:vec![]}}
#[test]
fn interrupted_download_publication_rolls_back_or_retains_commit_and_retry_reuses_video(){
 for point in ["download.after_copy_intent","download.after_publish","download.before_commit","download.after_commit"] {
  let (temp,path,root)=fixture();let source=temp.path().join("download.mp4");fs::write(&source,b"Synthetic video bytes").unwrap();
  crash(&path,&root,"download",point);file_moves::reconcile(&path).unwrap();
  let first=crate::download::import(&path,&root,&source,"Downloaded",&download_request()).unwrap();
  let second=crate::download::import(&path,&root,&source,"Downloaded",&download_request()).unwrap();assert!(second.reused);assert_eq!(first.id,second.id);
  let lib=db::load(&path,false).unwrap();assert_eq!(lib.items.len(),3);assert!(lib.items.iter().find(|i|i.id==first.id).unwrap().projects.contains(&1));
  assert_eq!(fs::read_dir(root.join("Downloads")).unwrap().count(),1);assert_eq!(import_pending(&path),0);assert_eq!(fs::read(source).unwrap(),b"Synthetic video bytes");
 }
}
#[test]
fn cancellation_before_commit_rolls_back_and_after_commit_reports_success(){
 for point in ["download.after_copy_intent","download.after_publish","download.before_commit","download.after_commit"] {
  let (temp,path,root)=fixture();let source=temp.path().join("download.mp4");fs::write(&source,b"Synthetic video bytes").unwrap();
  crash(&path,&root,"download_cancel",point);
  assert_eq!(fs::read_dir(root.join("Downloads")).unwrap().count(),if point=="download.after_commit"{1}else{0});
  assert_eq!(fs::read(source).unwrap(),b"Synthetic video bytes");
 }
}
