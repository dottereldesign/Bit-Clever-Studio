use crate::db::{self, err, Result};
use crate::history::Patch;
use rusqlite::{params, Connection, OptionalExtension};

// Remember removals separately because built-in and source-derived folders are reconstructed on load.
pub fn deleted(db: &Connection) -> Result<Vec<String>> {
    serde_json::from_str(&db::setting(db, "deleted_categories").unwrap_or_else(|| "[]".into())).map_err(err)
}
pub fn set_deleted(db: &Connection, names: &[String]) -> Result<()> {
    db::set_setting(db, "deleted_categories", &serde_json::to_string(names).map_err(err)?)
}
pub fn restore_name(db: &Connection, name: &str) -> Result<()> {
    let mut names = deleted(db)?;
    if !names.iter().any(|n|n==name) { return Ok(()); }
    names.retain(|n| n != name);
    set_deleted(db, &names)
}
fn canonical(name: &str) -> &str {
    match name {
        "Components / Hero" | "Components / Heroes" | "Components / Header" => "Components / Headers",
        "Components / Blog sections" | "Components / Blog archives" => "Components / Blog & News Sections",
        "Components / Logins" | "Components / Signups" => "Components / Logins & Signups",
        "Components / Product grids" => "Components / Products",
        _ => name,
    }
}
fn matches(name: &str, target: &str) -> bool { canonical(name).eq_ignore_ascii_case(target) && (name.starts_with("Components / ") || name == target) }
fn members(db: &Connection, name: &str) -> Result<Vec<i64>> {
    let mut stmt = db.prepare("SELECT i.id,i.source,c.name FROM items i LEFT JOIN item_categories c ON c.item_id=i.id").map_err(err)?;
    let rows = stmt.query_map([], |r| Ok((r.get::<_,i64>(0)?,r.get::<_,String>(1)?,r.get::<_,Option<String>>(2)?))).map_err(err)?;
    let mut ids=Vec::new();
    for row in rows {
        let (id,source,category)=row.map_err(err)?;
        let current=category.unwrap_or_else(||db::default_category(&source));
        if matches(&current,name) || db::design_memberships(db,id)?.iter().chain(db::component_memberships(db,id)?.iter()).any(|n|matches(n,name)) { ids.push(id); }
    }
    Ok(ids)
}
pub fn snapshot(db: &Connection, name: &str) -> Result<Vec<Patch>> {
    let mut patches=vec![Patch::DeletedFolders(deleted(db)?)];
    for id in members(db,name)? {
        patches.push(Patch::Category(id,db.query_row("SELECT name FROM item_categories WHERE item_id=?",[id],|r|r.get(0)).optional().map_err(err)?));
        patches.push(Patch::DesignFolders(id,db::design_memberships(db,id)?));
        patches.push(Patch::ComponentFolders(id,db::component_memberships(db,id)?));
        patches.push(Patch::CategoryArrivals(id,db::category_arrivals(db,id)?));
    }
    Ok(patches)
}
pub fn delete(db: &Connection, name: &str) -> Result<()> {
    if name.trim().is_empty() || name=="All Components" || name=="Components / Uncategorised" { return Err("This built-in collection cannot be deleted.".into()); }
    for id in members(db,name)? {
        db::materialize_design_folder(db,id)?;
        db::materialize_component_folder(db,id)?;
        for table in ["item_design_folders","item_component_folders","category_arrivals"] {
            let mut stmt=db.prepare(&format!("SELECT name FROM {table} WHERE item_id=?")).map_err(err)?;
            let names=stmt.query_map([id],|r|r.get::<_,String>(0)).map_err(err)?.collect::<std::result::Result<Vec<_>,_>>().map_err(err)?;
            for folder in names.into_iter().filter(|n|matches(n,name)) { db.execute(&format!("DELETE FROM {table} WHERE item_id=? AND name=?"),params![id,folder]).map_err(err)?; }
        }
        let source:String=db.query_row("SELECT source FROM items WHERE id=?",[id],|r|r.get(0)).map_err(err)?;
        let current:Option<String>=db.query_row("SELECT name FROM item_categories WHERE item_id=?",[id],|r|r.get(0)).optional().map_err(err)?;
        if matches(&current.unwrap_or_else(||db::default_category(&source)),name) {
            let next=if name.starts_with("Components / ") { "Components / Uncategorised".into() } else { db::design_memberships(db,id)?.first().cloned().unwrap_or_default() };
            db.execute("INSERT INTO item_categories VALUES(?,?) ON CONFLICT(item_id) DO UPDATE SET name=excluded.name",params![id,next]).map_err(err)?;
        }
    }
    let mut names=deleted(db)?;
    if !names.iter().any(|n|n==name) { names.push(name.to_owned()); }
    set_deleted(db,&names)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn apply(path:&std::path::Path,action:&str,value:&str) -> Result<()> {
        db::change(path,serde_json::from_value(serde_json::json!({"action":action,"value":value})).unwrap())
    }
    #[test]
    fn failed_folder_delete_rolls_back_memberships_and_history() {
        let temp=tempfile::tempdir().unwrap();let path=temp.path().join("failure.sqlite");db::init(&path).unwrap();
        let conn=db::connect(&path).unwrap();
        conn.execute_batch("INSERT INTO items(id,path,name,kind,source,added) VALUES(1,'one.png','One','image','Design/Branding',10);CREATE TRIGGER fail_folder_delete BEFORE INSERT ON settings WHEN NEW.key='deleted_categories' BEGIN SELECT RAISE(ABORT,'injected failure'); END;").unwrap();
        assert!(apply(&path,"delete_category","Branding").is_err());
        let lib=db::load(&path,false).unwrap();assert!(lib.deleted_categories.is_empty());assert_eq!(lib.items[0].category.as_deref(),Some("Branding"));assert!(lib.items[0].design_folders.is_empty());assert_eq!(lib.undo_count,0);
    }
    #[test]
    fn folder_deletion_preserves_items_and_other_memberships_and_roundtrips_history_and_backup() {
        let temp=tempfile::tempdir().unwrap();let path=temp.path().join("test.sqlite");db::init(&path).unwrap();
        db::connect(&path).unwrap().execute_batch("INSERT INTO items(id,path,name,kind,source,added) VALUES(1,'one.png','One','image','Design/Branding',10),(2,'two.png','Two','image','',10),(3,'three.png','Three','image','Design/Other',10);INSERT INTO item_categories VALUES(2,'Components / Heroes');INSERT INTO item_design_folders VALUES(1,'Other'),(2,'Illustration');INSERT INTO item_component_folders VALUES(2,'Components / Headers'),(2,'Components / Navbars');").unwrap();
        let before=db::load(&path,false).unwrap();
        apply(&path,"delete_category","Branding").unwrap();
        let after=db::load(&path,false).unwrap();assert_eq!(after.items.len(),3);assert!(after.deleted_categories.contains(&"Branding".into()));
        let one=after.items.iter().find(|i|i.id==1).unwrap();assert_eq!(one.category.as_deref(),Some("Other"));assert_eq!(one.design_folders,vec!["Other"]);
        apply(&path,"undo_organisation","Delete folder").unwrap();assert!(db::load(&path,false).unwrap().deleted_categories.is_empty());
        assert_eq!(db::load(&path,false).unwrap().items.iter().find(|i|i.id==1).unwrap().category,before.items.iter().find(|i|i.id==1).unwrap().category);
        apply(&path,"redo_organisation","").unwrap();
        apply(&path,"delete_category","Components / Headers").unwrap();let after=db::load(&path,false).unwrap();let two=after.items.iter().find(|i|i.id==2).unwrap();assert_eq!(two.component_folders,vec!["Components / Navbars"]);assert_eq!(two.design_folders,vec!["Illustration"]);
        apply(&path,"undo_organisation","Delete folder").unwrap();assert_eq!(db::load(&path,false).unwrap().items.iter().find(|i|i.id==2).unwrap().category.as_deref(),Some("Components / Heroes"));
        apply(&path,"redo_organisation","").unwrap();
        let backup=temp.path().join("backup.json");db::export(&path,backup.clone()).unwrap();let restored=temp.path().join("restored.sqlite");db::init(&restored).unwrap();db::import(&restored,backup).unwrap();assert_eq!(db::load(&restored,false).unwrap().deleted_categories,db::load(&path,false).unwrap().deleted_categories);
        // Recreating a deleted built-in folder restores an empty folder and can itself be undone.
        db::change(&path,serde_json::from_value(serde_json::json!({"action":"create_category","name":"Components / Headers"})).unwrap()).unwrap();assert!(!deleted(&db::connect(&path).unwrap()).unwrap().contains(&"Components / Headers".into()));
        apply(&path,"undo_organisation","Create category").unwrap();assert!(deleted(&db::connect(&path).unwrap()).unwrap().contains(&"Components / Headers".into()));
        for name in ["","All Components","Components / Uncategorised"] {assert!(apply(&path,"delete_category",name).is_err());}
        assert_eq!(db::load(&path,false).unwrap().items.len(),3);
    }
}
