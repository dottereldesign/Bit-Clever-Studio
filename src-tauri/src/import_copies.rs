//! Verified import copies are provisional until their metadata transaction commits.
use crate::{db::{err,Result},content::hash_file};
use rusqlite::{Connection,params};
use std::{fs,path::Path};
pub fn init(db:&Connection)->Result<()> {
 db.execute_batch("CREATE TABLE IF NOT EXISTS import_copy_journal(destination TEXT PRIMARY KEY,temporary TEXT NOT NULL,sha256 TEXT NOT NULL);").map_err(err)
}
pub fn record(db:&Connection,destination:&Path,temporary:&Path,hash:&str)->Result<()> {
 if destination.exists(){return Err("Import destination already exists.".into());}
 db.execute("INSERT INTO import_copy_journal VALUES(?,?,?)",params![destination.to_string_lossy(),temporary.to_string_lossy(),hash]).map_err(err)?;Ok(())
}
pub fn finish(db:&Connection)->Result<()> {db.execute("DELETE FROM import_copy_journal",[]).map_err(err)?;Ok(())}
fn entries(db:&Connection)->Result<Vec<(String,String,String)>> {
 let mut q=db.prepare("SELECT destination,temporary,sha256 FROM import_copy_journal").map_err(err)?;
 let rows=q.query_map([],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).map_err(err)?;
 rows.collect::<std::result::Result<Vec<_>,_>>().map_err(err)
}
pub fn reconcile(db:&Connection)->Result<()> {
 for (destination,temporary,hash) in entries(db)? {
  for name in [&destination,&temporary] {
   let path=Path::new(name);
   // A missing parent may be a disconnected drive, not a completed cleanup.
   if !path.parent().is_some_and(Path::is_dir){return Err("An interrupted import folder is unavailable. Reconnect it and retry.".into());}
   if !path.exists(){continue;}
   let referenced:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM items WHERE path=? OR thumbnail=?)",params![name,name],|r|r.get(0)).map_err(err)?;
   if referenced{return Err("An interrupted import copy is now referenced. It was preserved for review.".into());}
   if !path.is_file()||hash_file(path)?!=hash{return Err("An interrupted import copy has changed. It was preserved; resolve the conflict before retrying.".into());}
   fs::remove_file(path).map_err(err)?;
  }
  db.execute("DELETE FROM import_copy_journal WHERE destination=?",[destination]).map_err(err)?;
 }
 Ok(())
}
pub fn rebase(db:&Connection,old:&Path,folder:&Path)->Result<()> {
 for (destination,temporary,_) in entries(db)? {
  let rewrite=|s:&str|Path::new(s).strip_prefix(old).map(|p|folder.join(p).to_string_lossy().into_owned()).unwrap_or_else(|_|s.to_owned());
  db.execute("UPDATE import_copy_journal SET destination=?,temporary=? WHERE destination=?",params![rewrite(&destination),rewrite(&temporary),destination]).map_err(err)?;
 }Ok(())
}
