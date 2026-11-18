//! Development-only restore rehearsal. All writes are confined to a TempDir.
use super::*;
use serde_json::json;

pub(crate) fn verify_disposable_restore(source: &Path) -> Result<serde_json::Value> {
    let source = source.canonicalize().map_err(err)?;
    if source.join("Library Data/Library.sqlite").exists() {
        return Err("Choose a fresh exported backup, not a live library folder.".into());
    }
    let checked = verify(&source)?;
    if !checked.missing.is_empty() {
        return Err(
            "The backup lists unavailable originals; it cannot pass a complete restore rehearsal."
                .into(),
        );
    }
    let manifest_bytes = fs::read(manifest_path(&source)).map_err(err)?;
    let manifest: serde_json::Value = serde_json::from_slice(&manifest_bytes).map_err(err)?;
    let entries: Vec<Entry> = serde_json::from_value(manifest["files"].clone()).map_err(err)?;
    let temp = tempfile::tempdir().map_err(err)?;
    let copy = temp.path().join("rehearsal");
    fs::create_dir(&copy).map_err(err)?;
    // Copy only manifest-listed files. resolve_relative rejects escapes and symlinks
    // outside the source. All import/appearance/SQLite writes occur in this copy.
    for entry in &entries {
        let from = PathBuf::from(resolve_relative(&source, &entry.path)?);
        let to = PathBuf::from(resolve_relative(&copy, &entry.path)?);
        fs::create_dir_all(to.parent().ok_or("Missing copy parent")?).map_err(err)?;
        fs::copy(from, to).map_err(err)?;
    }
    let manifest_relative = manifest_path(&source)
        .strip_prefix(&source)
        .map_err(err)?
        .to_owned();
    let copied_manifest = copy.join(manifest_relative);
    fs::create_dir_all(copied_manifest.parent().ok_or("Missing manifest parent")?).map_err(err)?;
    fs::write(copied_manifest, manifest_bytes).map_err(err)?;
    // Recheck after copying to catch a changing source or an incomplete manifest.
    verify(&copy)?;
    let backup: Backup =
        serde_json::from_slice(&fs::read(metadata_path(&copy)).map_err(err)?).map_err(err)?;
    if !backup.portable {
        return Err("A full portable library export is required.".into());
    }
    let expected = backup.library;
    let restored_path = crate::managed_library::open(&copy)?;
    crate::managed_library::verify_live(&copy)?;
    let restored = db::load(&restored_path, false)?;
    let db = db::connect(&restored_path)?;
    let integrity: String = db
        .query_row("PRAGMA quick_check", [], |r| r.get(0))
        .map_err(err)?;
    if integrity != "ok" {
        return Err("Restored database integrity check failed.".into());
    }
    if db
        .prepare("PRAGMA foreign_key_check")
        .map_err(err)?
        .exists([])
        .map_err(err)?
    {
        return Err("Restored database contains broken references.".into());
    }
    for (name, before, after) in [
        ("items", expected.items.len(), restored.items.len()),
        ("projects", expected.projects.len(), restored.projects.len()),
        ("tags", expected.tags.len(), restored.tags.len()),
        (
            "project folders",
            expected.project_folders.len(),
            restored.project_folders.len(),
        ),
        (
            "project overviews",
            expected.project_overviews.len(),
            restored.project_overviews.len(),
        ),
        (
            "studio records",
            expected.studio.len(),
            restored.studio.len(),
        ),
        (
            "client records",
            expected.client_records.records.len(),
            restored.client_records.records.len(),
        ),
    ] {
        if before != after {
            return Err(format!("Restored {name} count differs from the backup."));
        }
    }
    // Compare curation independently of regenerated numeric IDs and organised paths.
    fn curation(library: &db::Library) -> Vec<String> {
        let mut rows: Vec<_> = library.items.iter().map(|i| {
            let names = |ids: &[i64], labels: &[db::Label]| {
                let mut names: Vec<_> = ids.iter().map(|id|labels.iter().find(|l|l.id==*id).map(|l|l.name.clone())).collect();
                names.sort(); names
            };
            let mut designs=i.design_folders.clone(); designs.sort();
            let mut components=i.component_folders.clone(); components.sort();
            json!({"name":i.name,"kind":i.kind,"source":i.source,"rating":i.rating,"project_only":i.project_only,
                "projects":names(&i.projects,&library.projects),"favourites":names(&i.favourites,&library.projects),
                "tags":names(&i.tags,&library.tags),"designs":designs,"components":components,
                "project_folder_count":i.project_folders.len()}).to_string()
        }).collect();
        rows.sort();
        rows
    }
    if curation(&expected) != curation(&restored) {
        return Err("Restored item curation differs from the backup.".into());
    }
    let preferences = crate::managed_library::preferences(&restored_path)?
        .ok_or("Restored preferences unavailable")?;
    if !preferences.is_object() {
        return Err("Restored preferences must be an object.".into());
    }
    // The generated database is temporary; never activate it or write an app pointer.
    let report = json!({"ok":true,"verified_files":checked.files,"items":restored.items.len(),"projects":restored.projects.len(),
        "project_folders":restored.project_folders.len(),"preferences":preferences.as_object().unwrap().len(),
        "database_integrity":"ok","temporary_restore":"removed on completion","source":"read-only"});
    drop(db);
    temp.close().map_err(err)?;
    Ok(report)
}

fn fixture() -> (tempfile::TempDir, PathBuf) {
    let temp = tempfile::tempdir().unwrap();
    let data = temp.path().join("data");
    fs::create_dir(&data).unwrap();
    let exports = temp.path().join("exports");
    fs::create_dir(&exports).unwrap();
    let path = data.join("library.sqlite");
    db::init(&path).unwrap();
    let media = data.join("sample.png");
    image::RgbImage::from_pixel(2, 2, image::Rgb([12, 34, 56]))
        .save(&media)
        .unwrap();
    let conn = db::connect(&path).unwrap();
    conn.execute(
        "INSERT INTO items(id,path,name,kind,source,added) VALUES(1,?,'Sample','image','Design',1)",
        [media.to_str().unwrap()],
    )
    .unwrap();
    conn.execute_batch("INSERT INTO projects VALUES(7,'Example'); INSERT INTO item_projects VALUES(1,7,1);
        INSERT INTO item_tags VALUES(1,1); INSERT INTO project_folders VALUES(11,7,NULL,'Research',0); INSERT INTO item_project_folders VALUES(1,11);").unwrap();
    let result = export_with_preferences(
        &path,
        &exports,
        "Rehearsal",
        json!({"sidebar-width":305,"home-tiles":[{"id":"project:7"}]}),
        &AtomicBool::new(false),
        &mut |_| {},
    )
    .unwrap();
    (temp, PathBuf::from(result.path))
}

#[test]
fn disposable_restore_preserves_source_and_rejects_corruption() {
    let (_temp, source) = fixture();
    let readme = fs::read_to_string(source.join("Read Me.txt")).unwrap();
    for term in [
        "My Business",
        "credentials",
        "download queues",
        "duplicate-recovery",
        "Undo/Redo history is not restored",
    ] {
        assert!(readme.contains(term), "Missing backup exclusion: {term}");
    }
    let metadata = fs::read(metadata_path(&source)).unwrap();
    let preferences = fs::read(preferences_path(&source)).unwrap();
    let report = verify_disposable_restore(&source).unwrap();
    assert_eq!(report["items"], 1);
    assert_eq!(report["project_folders"], 1);
    assert_eq!(fs::read(metadata_path(&source)).unwrap(), metadata);
    assert_eq!(fs::read(preferences_path(&source)).unwrap(), preferences);
    assert!(!source.join("Library Data/Library.sqlite").exists());
    verify(&source).unwrap();
    println!("RESTORE_VERIFICATION {report}");
    fs::write(preferences_path(&source), b"{}").unwrap();
    assert!(verify_disposable_restore(&source).is_err());
    fs::write(preferences_path(&source), preferences).unwrap();
    fs::write(source.join("INCOMPLETE.txt"), b"fixture").unwrap();
    assert!(verify_disposable_restore(&source).is_err());
}

#[test]
fn disposable_restore_rejects_missing_originals_and_invalid_metadata() {
    let (_temp, source) = fixture();
    let manifest_file = manifest_path(&source);
    let mut manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(&manifest_file).unwrap()).unwrap();
    manifest["missing"] = json!(["fixture unavailable"]);
    fs::write(&manifest_file, serde_json::to_vec(&manifest).unwrap()).unwrap();
    assert!(verify_disposable_restore(&source)
        .unwrap_err()
        .contains("unavailable originals"));
    manifest["missing"] = json!([]);
    let metadata = metadata_path(&source);
    let mut backup: serde_json::Value =
        serde_json::from_slice(&fs::read(&metadata).unwrap()).unwrap();
    backup["library"]["items"][0]["kind"] = json!("invalid");
    fs::write(&metadata, serde_json::to_vec(&backup).unwrap()).unwrap();
    for entry in manifest["files"].as_array_mut().unwrap() {
        if entry["path"] == "Library Data/library.json" {
            entry["sha256"] = json!(checksum(&metadata).unwrap());
            entry["bytes"] = json!(fs::metadata(&metadata).unwrap().len());
        }
    }
    fs::write(manifest_file, serde_json::to_vec(&manifest).unwrap()).unwrap();
    assert!(verify_disposable_restore(&source)
        .unwrap_err()
        .contains("Invalid item type"));
    assert!(!source.join("Library Data/Creating.sqlite").exists());
}

#[test]
#[ignore = "Read-only verification of an explicitly supplied exported backup"]
fn verify_supplied_backup() {
    let folder = std::env::var_os("DESIGN_VERIFY_BACKUP")
        .expect("Use npm run verify:restore -- --folder PATH");
    let report = verify_disposable_restore(Path::new(&folder)).expect("Restore rehearsal failed");
    println!("RESTORE_VERIFICATION {report}");
}
