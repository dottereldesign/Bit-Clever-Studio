//! Only marked, unlocked staging belonging to this application can be removed.
use crate::db::{err,Result};
use std::{fs::{self,File,OpenOptions},io::{Read,Write},path::{Path,PathBuf}};
const MARKER:&[u8]=b"Bit Clever download staging v1\n";
const LEASE:&str=".design-download-owner";
pub struct Staging {pub path:PathBuf,lease:Option<File>}
impl Staging {
 pub fn create(db:&Path)->Result<Self>{
  cleanup(db)?;
  let root=root(db)?;fs::create_dir_all(&root).map_err(err)?;
  let path=tempfile::Builder::new().prefix("owned-").tempdir_in(&root).map_err(err)?.keep();
  let mut lease=OpenOptions::new().read(true).write(true).create_new(true).open(path.join(LEASE)).map_err(err)?;
  lease.try_lock().map_err(err)?;lease.write_all(MARKER).map_err(err)?;lease.sync_all().map_err(err)?;
  Ok(Self{path,lease:Some(lease)})
 }
}
impl Drop for Staging {fn drop(&mut self){self.lease.take();let _=remove_owned(&self.path);}}
fn root(db:&Path)->Result<PathBuf>{Ok(db.parent().ok_or("Missing library data folder")?.join("video-downloads"))}
fn linked(metadata:&fs::Metadata)->bool {
 #[cfg(windows)] {use std::os::windows::fs::MetadataExt;metadata.file_attributes()&0x400!=0}
 #[cfg(not(windows))] {metadata.file_type().is_symlink()}
}
fn remove_owned(path:&Path)->Result<bool>{
 if !path.is_dir(){return Ok(false);}
 if linked(&fs::symlink_metadata(path).map_err(err)?){return Ok(false);}
 let lease_path=path.join(LEASE);
 if fs::symlink_metadata(&lease_path).map(|m|linked(&m)).unwrap_or(true){return Ok(false);}
 let mut lease=match OpenOptions::new().read(true).write(true).open(&lease_path){Ok(file)=>file,Err(_)=>return Ok(false)};
 if lease.try_lock().is_err(){return Ok(false);}
 let mut marker=vec![];Read::by_ref(&mut lease).take(128).read_to_end(&mut marker).map_err(err)?;
 if marker!=MARKER{return Ok(false);}
 // Never follow a link/reparse point to files outside the owned directory.
 for entry in walkdir::WalkDir::new(path).follow_links(false){let entry=entry.map_err(err)?;if linked(&fs::symlink_metadata(entry.path()).map_err(err)?){return Ok(false);}}
 // Keep the marker lock through deletion of payloads, so another reaper cannot race.
 for entry in fs::read_dir(path).map_err(err)?{let entry=entry.map_err(err)?;if entry.file_name()==LEASE{continue;}if entry.file_type().map_err(err)?.is_dir(){fs::remove_dir_all(entry.path()).map_err(err)?;}else{fs::remove_file(entry.path()).map_err(err)?;}}
 drop(lease);
 fs::remove_file(lease_path).map_err(err)?;fs::remove_dir(path).map_err(err)?;Ok(true)
}
pub fn cleanup(db:&Path)->Result<usize>{
 let root=root(db)?;if !root.exists(){return Ok(0);}let mut count=0;
 for entry in fs::read_dir(root).map_err(err)?{let entry=entry.map_err(err)?;if entry.file_name().to_string_lossy().starts_with("owned-")&&remove_owned(&entry.path())?{count+=1;}}
 Ok(count)
}
