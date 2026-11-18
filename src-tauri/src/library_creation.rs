//! Rebuild only our unpublished setup database; never delete unknown setup files.
use crate::db::{err,Result};
use std::{fs::{self,File,OpenOptions},io::Write,path::{Path,PathBuf}};
const MARKER:&str="Bit Clever library creation v1\n";
pub fn lock(folder:&Path)->Result<File>{
 let file=OpenOptions::new().create(true).truncate(false).read(true).write(true).open(folder.join("Library Data/.creation.lock")).map_err(err)?;
 file.try_lock().map_err(|_|"Library setup is already running in another process.".to_owned())?;Ok(file)
}
pub fn cleanup(folder:&Path)->Result<()> {
 let dir=folder.join("Library Data/.creating");if !dir.exists(){return Ok(());}
 if fs::symlink_metadata(&dir).map_err(err)?.file_type().is_symlink(){return Err("Library setup folder is a link; it was preserved.".into());}
 let entries:Vec<_>=fs::read_dir(&dir).map_err(err)?.collect::<std::io::Result<_>>().map_err(err)?;
 if entries.is_empty(){fs::remove_dir(dir).map_err(err)?;return Ok(());}
 if fs::read_to_string(dir.join("owner")).ok().as_deref()!=Some(MARKER){return Err("Unrecognised unfinished library setup was preserved for review.".into());}
 for entry in &entries {
  let name=entry.file_name();let name=name.to_string_lossy();
  if !["owner","Library.sqlite","Library.sqlite-wal","Library.sqlite-shm","Library.sqlite-journal"].contains(&name.as_ref())||!entry.file_type().map_err(err)?.is_file(){return Err("Unexpected files in unfinished library setup were preserved for review.".into());}
 }
 // Retain ownership evidence if cleanup itself is interrupted.
 for entry in entries {if entry.file_name()!="owner"{fs::remove_file(entry.path()).map_err(err)?;}}
 fs::remove_file(dir.join("owner")).map_err(err)?;
 fs::remove_dir(dir).map_err(err)
}
pub fn prepare(folder:&Path)->Result<PathBuf>{
 cleanup(folder)?;
 let dir=folder.join("Library Data/.creating");fs::create_dir(&dir).map_err(err)?;
 let mut marker=OpenOptions::new().write(true).create_new(true).open(dir.join("owner")).map_err(err)?;
 marker.write_all(MARKER.as_bytes()).map_err(err)?;marker.sync_all().map_err(err)?;
 Ok(dir.join("Library.sqlite"))
}
