use crate::{db,content,managed_library,file_moves,library_export};
use std::{fs,path::{Path,PathBuf},process::{Command,Stdio},time::{Instant,Duration}};
fn fixture()->(tempfile::TempDir,PathBuf,PathBuf){
 let temp=tempfile::tempdir().unwrap();let root=temp.path().canonicalize().unwrap();fs::create_dir(root.join("data")).unwrap();let path=root.join("data/library.sqlite");db::init(&path).unwrap();
 let original=root.join("image.png");image::RgbImage::from_pixel(3,3,image::Rgb([11,22,33])).save(&original).unwrap();
 db::connect(&path).unwrap().execute("INSERT INTO items(path,name,kind,added) VALUES(?,'Original','image',1)",[original.to_str().unwrap()]).unwrap();
 (temp,path,root)
}
fn worker_command(kind:&str,path:&Path,root:&Path,point:&str)->Command {
 let mut command=Command::new(std::env::current_exe().unwrap());
 #[cfg(windows)] {use std::os::windows::process::CommandExt;command.creation_flags(0x08000000);}
 command.args(["--exact","full_recovery_tests::worker","--ignored","--nocapture"]).env("DESIGN_RECOVERY_KIND",kind).env("DESIGN_RECOVERY_DB",path).env("DESIGN_RECOVERY_ROOT",root).env("DESIGN_TEST_CRASH_POINT",point);command
}
fn crash(kind:&str,path:&Path,root:&Path,point:&str){
 let result=worker_command(kind,path,root,point).output().unwrap();assert_eq!(result.status.code(),Some(86),"{point}: {} {}",String::from_utf8_lossy(&result.stdout),String::from_utf8_lossy(&result.stderr));
}
fn thumbnail_input(root:&Path)->content::Capture {serde_json::from_value(serde_json::json!({"url":"https://example.test/site","destination":"websites","thumbnail":{"path":root.join("image.png"),"name":"image.png"}})).unwrap()}
#[test] #[ignore="Isolated fault-injection worker"]
fn worker(){
 let root=PathBuf::from(std::env::var_os("DESIGN_RECOVERY_ROOT").unwrap());let path=PathBuf::from(std::env::var_os("DESIGN_RECOVERY_DB").unwrap());
 match std::env::var("DESIGN_RECOVERY_KIND").unwrap().as_str(){
  "creation"=>{managed_library::activate(&root.join("config"),&root.join("backup")).unwrap();},
  "thumbnail"=>{content::capture(&path,thumbnail_input(&root)).unwrap();},
  "wal"=>{let mut conn=db::connect(&path).unwrap();let tx=conn.transaction().unwrap();tx.execute("INSERT INTO projects(name) VALUES('Atomic')",[]).unwrap();file_moves::checkpoint("wal.before_commit");tx.commit().unwrap();file_moves::checkpoint("wal.after_commit");},
  #[cfg(windows)] "download_owner"=>{download_owner(&path,&root);},
  _=>panic!("Unknown test operation"),
 }
 panic!("Checkpoint not reached")
}
#[test]
fn generated_thumbnail_interruptions_keep_original_and_committed_references(){
 for point in ["thumbnail.after_intent","thumbnail.after_publish","thumbnail.before_commit","thumbnail.after_commit"] {
  let (_temp,path,root)=fixture();let original=fs::read(root.join("image.png")).unwrap();
  crash("thumbnail",&path,&root,point);file_moves::reconcile(&path).unwrap();
  let lib=db::load(&path,false).unwrap();assert_eq!(lib.items.len(),if point=="thumbnail.after_commit"{2}else{1});
  assert_eq!(fs::read_dir(path.parent().unwrap().join("link-thumbnails")).unwrap().count(),if point=="thumbnail.after_commit"{1}else{0});
  if point!="thumbnail.after_commit"{content::capture(&path,thumbnail_input(&root)).unwrap();}
  let saved=db::load(&path,false).unwrap().items.into_iter().find(|i|i.kind=="link").unwrap();assert!(image::open(saved.thumbnail).is_ok());
  assert_eq!(fs::read(root.join("image.png")).unwrap(),original);
 }
}
#[test]
fn replacement_thumbnail_survives_crash_and_history_roundtrip(){
 for point in ["thumbnail.after_publish","thumbnail.before_commit","thumbnail.after_commit"] {
  let (_temp,path,root)=fixture();
  let mut input=thumbnail_input(&root);input.destination="links".into();
  content::capture(&path,input).unwrap();
  let old=db::load(&path,false).unwrap().items.into_iter().find(|i|i.kind=="link").unwrap().thumbnail;
  let bytes=fs::read(&old).unwrap();
  image::RgbImage::from_pixel(4,4,image::Rgb([99,88,77])).save(root.join("image.png")).unwrap();
  crash("thumbnail",&path,&root,point);file_moves::reconcile(&path).unwrap();
  let current=db::load(&path,false).unwrap().items.into_iter().find(|i|i.kind=="link").unwrap().thumbnail;
  assert_eq!(fs::read(&old).unwrap(),bytes);
  if point=="thumbnail.after_commit" {
   assert_ne!(current,old);assert!(image::open(&current).is_ok());
   let conn=db::connect(&path).unwrap();crate::history::undo(&conn).unwrap();
   assert_eq!(db::load(&path,false).unwrap().items.into_iter().find(|i|i.kind=="link").unwrap().thumbnail,old);
   crate::history::redo(&conn).unwrap();
   assert_eq!(db::load(&path,false).unwrap().items.into_iter().find(|i|i.kind=="link").unwrap().thumbnail,current);
  }else{assert_eq!(current,old);}
 }
}
fn creation_fixture()->(tempfile::TempDir,PathBuf,PathBuf){
 let (temp,path,root)=fixture();fs::create_dir(root.join("config")).unwrap();
 library_export::export(&path,&root,"backup",&std::sync::atomic::AtomicBool::new(false),&mut |_|{}).unwrap();
 fs::write(root.join("config/active-library.json"),b"\"old selection\"").unwrap();(temp,path,root)
}
#[test]
fn library_creation_and_activation_recover_at_each_boundary(){
 for point in ["creation.after_schema","creation.after_import","creation.before_publish","creation.after_publish","creation.after_organise","creation.before_activate","creation.after_activate"] {
  let (_temp,path,root)=creation_fixture();crash("creation",&path,&root,point);
  if point!="creation.after_activate"{assert_eq!(fs::read(root.join("config/active-library.json")).unwrap(),b"\"old selection\"");}
  managed_library::activate(&root.join("config"),&root.join("backup")).unwrap();
  let opened=managed_library::selected_db(&root.join("config")).unwrap();let lib=db::load(&opened,false).unwrap();assert_eq!(lib.items.len(),1);assert!(Path::new(&lib.items[0].path).is_file());
  let conn=db::connect(&opened).unwrap();assert_eq!(conn.query_row("PRAGMA integrity_check",[],|r|r.get::<_,String>(0)).unwrap(),"ok");assert!(!root.join("backup/Library Data/.creating").exists());
 }
}
#[test]
fn torn_unpublished_setup_rebuilds_but_corrupt_published_database_is_preserved(){
 let (_temp,path,root)=creation_fixture();crash("creation",&path,&root,"creation.after_schema");
 fs::write(root.join("backup/Library Data/.creating/Library.sqlite"),b"torn temporary database").unwrap();
 managed_library::activate(&root.join("config"),&root.join("backup")).unwrap();
 let db=root.join("backup/Library Data/Library.sqlite");fs::write(&db,b"corrupted published database").unwrap();let pointer=fs::read(root.join("config/active-library.json")).unwrap();
 assert!(managed_library::activate(&root.join("config"),&root.join("backup")).is_err());assert_eq!(fs::read(db).unwrap(),b"corrupted published database");assert_eq!(fs::read(root.join("config/active-library.json")).unwrap(),pointer);
}
#[test]
fn unknown_setup_files_and_concurrent_setup_are_preserved(){
 let (_temp,_path,root)=creation_fixture();let folder=root.join("backup");
 let lock=crate::library_creation::lock(&folder).unwrap();assert!(managed_library::open(&folder).is_err());drop(lock);
 let stage=folder.join("Library Data/.creating");fs::create_dir(&stage).unwrap();fs::write(stage.join("personal.txt"),b"Keep").unwrap();
 assert!(managed_library::open(&folder).is_err());assert_eq!(fs::read(stage.join("personal.txt")).unwrap(),b"Keep");
}
#[test]
fn sqlite_full_and_process_exit_preserve_transaction_boundaries(){
 let (_temp,path,root)=fixture();let mut conn=db::connect(&path).unwrap();
 assert_eq!(conn.query_row("PRAGMA synchronous",[],|r|r.get::<_,i64>(0)).unwrap(),2);
 let pages:i64=conn.query_row("PRAGMA page_count",[],|r|r.get(0)).unwrap();conn.pragma_update(None,"max_page_count",pages).unwrap();
 {let tx=conn.transaction().unwrap();let result=tx.execute("INSERT INTO settings(key,value) VALUES('oversized',zeroblob(1048576))",[]);assert!(result.is_err());}
 assert_eq!(db::setting(&conn,"oversized"),None);drop(conn);
 crash("wal",&path,&root,"wal.before_commit");assert!(db::load(&path,false).unwrap().projects.is_empty());
 crash("wal",&path,&root,"wal.after_commit");assert_eq!(db::load(&path,false).unwrap().projects[0].name,"Atomic");
 assert_eq!(db::connect(&path).unwrap().query_row("PRAGMA integrity_check",[],|r|r.get::<_,String>(0)).unwrap(),"ok");
}
#[test]
fn unavailable_library_does_not_replace_active_selection(){
 let (_temp,_path,root)=creation_fixture();managed_library::activate(&root.join("config"),&root.join("backup")).unwrap();
 let pointer=fs::read(root.join("config/active-library.json")).unwrap();fs::rename(root.join("backup"),root.join("disconnected")).unwrap();
 assert!(managed_library::selected_db(&root.join("config")).is_err());assert_eq!(fs::read(root.join("config/active-library.json")).unwrap(),pointer);
 fs::rename(root.join("disconnected"),root.join("backup")).unwrap();assert!(managed_library::selected_db(&root.join("config")).is_ok());
}
#[cfg(windows)]
fn locked_marker(path:&Path)->fs::File {use std::os::windows::fs::OpenOptionsExt;let file=fs::OpenOptions::new().read(true).write(true).create_new(true).share_mode(0).open(path).unwrap();file.sync_all().unwrap();file}
#[cfg(windows)]
fn download_owner(path:&Path,root:&Path){
 let stage=crate::download_staging::Staging::create(path).unwrap();fs::write(stage.path.join("video.part"),b"unfinished download").unwrap();
 let mut command=Command::new(std::env::current_exe().unwrap());command.args(["--exact","full_recovery_tests::downloader_worker","--ignored","--nocapture"]).env("DESIGN_TREE_ROOT",root);
 let (_tree,mut child)=crate::download_process::Tree::spawn(&command).unwrap();
 let started=Instant::now();while !root.join("grandchild.lock").exists()&&started.elapsed()<Duration::from_secs(15){if let Some(status)=child.try_wait().unwrap(){panic!("Supervisor exited: {status}");}std::thread::sleep(Duration::from_millis(20));}
 assert!(root.join("grandchild.lock").exists());fs::write(root.join("owner.ready"),stage.path.to_string_lossy().as_bytes()).unwrap();loop{std::thread::sleep(Duration::from_secs(1));}
}
#[cfg(windows)]
#[test] #[ignore="Isolated fake downloader worker"]
fn downloader_worker(){
 use std::os::windows::process::CommandExt;
 let root=PathBuf::from(std::env::var_os("DESIGN_TREE_ROOT").unwrap());let _file=locked_marker(&root.join("child.lock"));
 let mut child=Command::new(std::env::current_exe().unwrap()).args(["--exact","full_recovery_tests::grandchild_worker","--ignored"]).creation_flags(0x08000000).env("DESIGN_TREE_ROOT",root).stdout(Stdio::null()).stderr(Stdio::null()).spawn().unwrap();let _=child.wait();
}
#[cfg(windows)]
#[test] #[ignore="Isolated fake transcoder worker"]
fn grandchild_worker(){let root=PathBuf::from(std::env::var_os("DESIGN_TREE_ROOT").unwrap());let _file=locked_marker(&root.join("grandchild.lock"));loop{std::thread::sleep(Duration::from_secs(1));}}
struct ChildGuard(std::process::Child);impl Drop for ChildGuard{fn drop(&mut self){let _=self.0.kill();let _=self.0.wait();}}
#[cfg(windows)]
#[test]
fn app_death_kills_download_process_tree_and_reaps_only_abandoned_owned_staging(){
 let (_temp,path,root)=fixture();let mut owner=ChildGuard(worker_command("download_owner",&path,&root,"").stdout(Stdio::null()).stderr(Stdio::null()).spawn().unwrap());
 let started=Instant::now();while !root.join("owner.ready").exists()&&started.elapsed()<Duration::from_secs(20){std::thread::sleep(Duration::from_millis(20));}
 assert!(root.join("owner.ready").exists(),"Download worker did not start");assert_eq!(crate::download_staging::cleanup(&path).unwrap(),0);
 let unknown=path.parent().unwrap().join("video-downloads/legacy-unknown");fs::create_dir(&unknown).unwrap();fs::write(unknown.join("keep"),b"unidentified").unwrap();
 owner.0.kill().unwrap();owner.0.wait().unwrap();
 let started=Instant::now();while started.elapsed()<Duration::from_secs(10) {if fs::File::open(root.join("child.lock")).is_ok()&&fs::File::open(root.join("grandchild.lock")).is_ok(){break;}std::thread::sleep(Duration::from_millis(20));}
 assert!(fs::File::open(root.join("child.lock")).is_ok());assert!(fs::File::open(root.join("grandchild.lock")).is_ok());
 assert_eq!(crate::download_staging::cleanup(&path).unwrap(),1);assert_eq!(fs::read(unknown.join("keep")).unwrap(),b"unidentified");
 let stage=crate::download_staging::Staging::create(&path).unwrap();let new_path=stage.path.clone();assert_eq!(crate::download_staging::cleanup(&path).unwrap(),0);drop(stage);assert!(!new_path.exists());
}
