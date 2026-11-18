use crate::{content, db, library_export, managed_library};
use serde_json::{json, Value};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::AtomicBool,
};

fn change(path: &Path, value: Value) {
    db::change(path, serde_json::from_value(value).unwrap()).unwrap();
}
fn snapshot(path: &Path) -> Value {
    serde_json::to_value(db::load(path, false).unwrap()).unwrap()
}

#[test]
fn recovery_contract_round_trip() {
    let temp = tempfile::tempdir().unwrap();
    let base = temp.path().canonicalize().unwrap();
    let original = base.join("original");
    fs::create_dir_all(original.join("Library Data")).unwrap();
    fs::create_dir_all(original.join("Design Inspo")).unwrap();
    let path = original.join("Library Data/Library.sqlite");
    db::init(&path).unwrap();
    let one = original.join("Design Inspo/one.png");
    let two = original.join("Design Inspo/two.png");
    image::RgbImage::from_pixel(2, 2, image::Rgb([12, 34, 56]))
        .save(&one)
        .unwrap();
    fs::copy(&one, &two).unwrap();
    let original_bytes = fs::read(&two).unwrap();
    {
        let c = db::connect(&path).unwrap();
        db::set_setting(&c, "managed_root", original.to_str().unwrap()).unwrap();
        c.execute_batch(
            "INSERT INTO projects VALUES(7,'Contract project');
            INSERT INTO project_folders VALUES(11,7,NULL,'Research',0),(12,7,11,'References',0);",
        )
        .unwrap();
        for (id, file) in [(1, &one), (2, &two)] {
            c.execute("INSERT INTO items(id,path,name,kind,source,added) VALUES(?,?,?,'image','Design',1700000000000)", rusqlite::params![id,file.to_str().unwrap(),format!("Image {id}")]).unwrap();
        }
        c.execute_batch("INSERT INTO item_projects VALUES(2,7,1); INSERT INTO item_project_folders VALUES(2,12); INSERT INTO item_tags VALUES(2,1);
            INSERT INTO items(id,path,name,kind,source,added,project_only,rating) VALUES(3,'https://example.test/private','Private link','link','',1700000000000,1,4);
            INSERT INTO item_projects VALUES(3,7,0);").unwrap();
    }
    change(
        &path,
        json!({"action":"set_category","ids":[2],"value":"Illustration"}),
    );
    change(
        &path,
        json!({"action":"set_category","ids":[2],"value":"Components / Products"}),
    );
    change(
        &path,
        json!({"action":"update_project_overview","target":7,"value":json!({"field":"moodboard","value":[{"id":"reference","kind":"reference","item_id":2,"text":"","color":"#123456","x":0,"y":0,"width":200,"height":200}]}).to_string()}),
    );
    change(
        &path,
        json!({"action":"update_client_records","target":7,"value":json!({"type":"record","id":"note","patch":{"kind":"meeting","title":"Contract meeting","files":[2]}}).to_string()}),
    );
    content::remove_duplicates(&path, &original, 1, vec![2]).unwrap();
    let merged = snapshot(&path);
    assert_eq!(merged["items"].as_array().unwrap().len(), 2);
    assert_eq!(merged["items"][0]["project_folders"], json!([12]));
    assert_eq!(merged["project_overviews"][0]["moodboard"][0]["item_id"], 1);
    assert_eq!(merged["client_records"]["records"][0]["files"], json!([1]));

    // Move the whole disposable library while the duplicate remains recoverable.
    let moved = base.join("moved");
    fs::rename(&original, &moved).unwrap();
    let path = moved.join("Library Data/Library.sqlite");
    managed_library::rebase(&path, &moved).unwrap();
    let restored_file = moved.join("Design Inspo/two.png");
    fs::write(&restored_file, b"collision must survive").unwrap();
    let before_failure = snapshot(&path);
    assert!(content::restore(&path, 2)
        .unwrap_err()
        .contains("already exists"));
    assert_eq!(snapshot(&path), before_failure);
    assert_eq!(content::recovery(&path).unwrap().len(), 1);
    assert_eq!(fs::read(&restored_file).unwrap(), b"collision must survive");
    fs::remove_file(&restored_file).unwrap();
    content::restore(&path, 2).unwrap();
    assert_eq!(fs::read(&restored_file).unwrap(), original_bytes);
    assert!(content::recovery(&path).unwrap().is_empty());

    // Preference commits survive derivative-file failures and can be repaired.
    let prefs = json!({"sidebar-width":305,"folder-labels":{"Illustration":"Sketches"},"home-tiles":[{"id":"project:7"}],"colour-history":[{"hex":"#123456","projects":["project:7"]}]});
    let obstruction = moved.join("Browse Library.html");
    fs::create_dir(&obstruction).unwrap();
    managed_library::save_preferences(&path, &prefs).unwrap();
    assert_eq!(
        managed_library::preferences(&path).unwrap(),
        Some(prefs.clone())
    );
    assert!(db::load(&path, false)
        .unwrap()
        .maintenance_warning
        .is_some());
    fs::remove_dir(&obstruction).unwrap();
    managed_library::after_commit(&path, false);
    assert!(db::load(&path, false)
        .unwrap()
        .maintenance_warning
        .is_none());

    // Inject a SQLite persistence failure without modifying application code.
    let connection = db::connect(&path).unwrap();
    connection.execute_batch("CREATE TRIGGER reject_preferences BEFORE UPDATE ON settings WHEN NEW.key='appearance_preferences' BEGIN SELECT RAISE(ABORT,'fixture save failure'); END;").unwrap();
    assert!(managed_library::save_preferences(&path, &json!({"sidebar-width":400})).is_err());
    assert_eq!(
        managed_library::preferences(&path).unwrap(),
        Some(prefs.clone())
    );
    connection
        .execute_batch("DROP TRIGGER reject_preferences;")
        .unwrap();
    drop(connection);
    managed_library::save_preferences(&path, &prefs).unwrap();

    let destination = base.join("exports");
    fs::create_dir(&destination).unwrap();
    let output = library_export::export_with_preferences(
        &path,
        &destination,
        "Contract",
        prefs,
        &AtomicBool::new(false),
        &mut |_| {},
    )
    .unwrap();
    let backup = PathBuf::from(output.path);
    library_export::verify(&backup).unwrap();
    let relocated_backup = base.join("relocated-backup");
    fs::rename(&backup, &relocated_backup).unwrap();
    let rehearsal = library_export::backup_verification_tests::verify_disposable_restore(&relocated_backup).unwrap();
    assert_eq!(rehearsal["items"], 3);
    assert_eq!(rehearsal["project_folders"], 2);

    // Reject incomplete activation before replacing the active-library pointer.
    let config = base.join("config");
    fs::create_dir(&config).unwrap();
    let pointer = config.join("active-library.json");
    let pointer_bytes = serde_json::to_vec(&moved).unwrap();
    fs::write(&pointer, &pointer_bytes).unwrap();
    let incomplete = relocated_backup.join("INCOMPLETE.txt");
    fs::write(&incomplete, b"interrupted backup fixture").unwrap();
    assert!(managed_library::activate(&config, &relocated_backup).is_err());
    assert_eq!(fs::read(&pointer).unwrap(), pointer_bytes);
    assert!(!relocated_backup
        .join("Library Data/Creating.sqlite")
        .exists());
    assert!(!relocated_backup
        .join("Library Data/Library.sqlite")
        .exists());
    fs::remove_file(incomplete).unwrap();
    library_export::verify(&relocated_backup).unwrap();
    let metadata = library_export::metadata_path(&relocated_backup);
    let target = base.join("target.sqlite");
    db::init(&target).unwrap();
    {
        let c = db::connect(&target).unwrap();
        c.execute_batch("INSERT INTO projects VALUES(7,'Unrelated'); INSERT INTO items(id,path,name,kind,added) VALUES(1,'https://example.test/unrelated','Unrelated','link',1);").unwrap();
    }
    // A failure after label insertion must roll back all database changes.
    let mut invalid: Value = serde_json::from_slice(&fs::read(&metadata).unwrap()).unwrap();
    invalid["library"]["items"][1]["kind"] = json!("invalid-kind");
    let malformed = metadata.with_file_name("invalid.json");
    fs::write(&malformed, serde_json::to_vec(&invalid).unwrap()).unwrap();
    let before = snapshot(&target);
    assert!(db::import(&target, malformed)
        .unwrap_err()
        .contains("Invalid item type"));
    assert_eq!(snapshot(&target), before);
    db::import(&target, metadata.clone()).unwrap();
    let imported = snapshot(&target);
    let restored_preferences =
        library_export::restore_preferences(&relocated_backup, &target).unwrap();
    let project = db::load(&target, false)
        .unwrap()
        .projects
        .into_iter()
        .find(|p| p.name == "Contract project")
        .unwrap()
        .id;
    assert_ne!(project, 7);
    assert_eq!(
        restored_preferences["home-tiles"][0]["id"],
        format!("project:{project}")
    );
    assert_eq!(
        restored_preferences["colour-history"][0]["projects"],
        json!([format!("project:{project}")])
    );
    let lib = db::load(&target, false).unwrap();
    assert_eq!(lib.items.len(), 4);
    let keeper = lib.items.iter().find(|i| i.name == "Image 1").unwrap();
    assert_eq!(keeper.projects, vec![project]);
    assert_eq!(keeper.favourites, vec![project]);
    assert_eq!(
        lib.project_overviews[0].moodboard.as_ref().unwrap()[0].item_id,
        Some(keeper.id)
    );
    assert_eq!(lib.client_records.records[0].files, vec![keeper.id]);
    assert_eq!(fs::read(&keeper.path).unwrap(), original_bytes);
    db::import(&target, metadata).unwrap();
    assert_eq!(
        db::load(&target, false).unwrap().items.len(),
        4,
        "Repeated import must not duplicate items"
    );
    let private = lib.items.iter().find(|i| i.name == "Private link").unwrap();
    assert!(private.project_only);
    assert_eq!(private.rating, Some(4));
    change(
        &target,
        json!({"action":"rate_links","ids":[private.id],"value":"5"}),
    );
    let rated = snapshot(&target);
    change(&target, json!({"action":"undo_organisation"}));
    let undone = snapshot(&target);
    change(&target, json!({"action":"redo_organisation"}));
    let redone = snapshot(&target);
    if let Some(output) = std::env::var_os("DESIGN_CONTRACT_OUTPUT") {
        let mut wire = json!({"imported":imported,"rated":rated,"undone":undone,"redone":redone,"preferences":restored_preferences});
        fn anonymise(value: &mut Value, base: &Path) {
            match value {
                Value::String(s) => {
                    if let Ok(relative) = Path::new(s).strip_prefix(base) {
                        *s = format!(
                            "C:/SyntheticContract/{}",
                            relative.to_string_lossy().replace('\\', "/")
                        );
                    }
                }
                Value::Array(values) => {
                    for v in values {
                        anonymise(v, base)
                    }
                }
                Value::Object(values) => {
                    for v in values.values_mut() {
                        anonymise(v, base)
                    }
                }
                _ => {}
            }
        }
        anonymise(&mut wire, &base);
        fs::write(output, serde_json::to_vec_pretty(&wire).unwrap()).unwrap();
    }
}
