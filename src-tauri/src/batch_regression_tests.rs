use super::*;
use serde_json::json;
use std::{fs,path::Path};
fn edit(p:&Path,v:serde_json::Value){db::change(p,serde_json::from_value(v).unwrap()).unwrap();}
#[test] fn audit_duplicate_memberships_and_references(){
 let t=tempfile::tempdir().unwrap();let p=t.path().join("db.sqlite");db::init(&p).unwrap();let a=t.path().join("a.png");let b=t.path().join("b.png");image::RgbImage::from_pixel(2,2,image::Rgb([12,34,56])).save(&a).unwrap();fs::copy(&a,&b).unwrap();let c=db::connect(&p).unwrap();
 c.execute_batch("INSERT INTO projects VALUES(1,'Example');").unwrap();for(id,path)in [(1,&a),(2,&b)]{c.execute("INSERT INTO items(id,path,name,kind,added)VALUES(?,?,'Fixture','image',1)",rusqlite::params![id,path.to_str().unwrap()]).unwrap();c.execute("INSERT INTO item_projects VALUES(?,1,0)",[id]).unwrap();}
 edit(&p,json!({"action":"set_category","ids":[2],"value":"Illustration"}));edit(&p,json!({"action":"set_category","ids":[2],"value":"Components / Products"}));
 edit(&p,json!({"action":"update_project_overview","target":1,"value":json!({"field":"moodboard","value":[{"id":"ref","kind":"reference","item_id":2,"text":"","color":"#123456","x":0,"y":0,"width":200,"height":200}]}).to_string()}));
 edit(&p,json!({"action":"update_client_records","target":1,"value":json!({"type":"record","id":"note","patch":{"kind":"meeting","title":"Fixture meeting","files":[2]}}).to_string()}));
 content::remove_duplicates(&p,t.path(),1,vec![2]).unwrap();let l=db::load(&p,false).unwrap();println!("AUDIT_DUPLICATE: {}",json!({"active":l.items.len(),"design":l.items[0].design_folders,"component":l.items[0].component_folders,"moodboard_item":l.project_overviews[0].moodboard.as_ref().unwrap()[0].item_id,"client_files":l.client_records.records[0].files}));assert!(l.items[0].design_folders.contains(&"Illustration".to_owned()));assert!(l.items[0].component_folders.contains(&"Components / Products".to_owned()));assert_eq!(l.project_overviews[0].moodboard.as_ref().unwrap()[0].item_id,Some(1));assert_eq!(l.client_records.records[0].files,vec![1]);
}
#[test] fn audit_media_cache_isolated_between_libraries(){
 let t=tempfile::tempdir().unwrap();let cache=t.path().join("cache");let mut previews=vec![];
 for (name,pixel) in [("a",[255,0,0]),("b",[0,0,255])]{let p=t.path().join(format!("{name}.sqlite"));db::init(&p).unwrap();let source=t.path().join(format!("{name}.bmp"));image::RgbImage::from_pixel(8,8,image::Rgb(pixel)).save(&source).unwrap();db::connect(&p).unwrap().execute("INSERT INTO items(id,path,name,kind,added,modified)VALUES(1,?,'Fixture','image',1,123)",[source.to_str().unwrap()]).unwrap();previews.push(library::media(&p,&cache,Path::new("unused-ffmpeg"),1,"thumbnail").unwrap());}
 println!("AUDIT_CACHE: paths_equal={}",previews[0]==previews[1]);assert_ne!(previews[0],previews[1],"Different library media resolve to the same cached image");
}
#[test] fn audit_project_only_link_capture(){
 let t=tempfile::tempdir().unwrap();let p=t.path().join("db.sqlite");db::init(&p).unwrap();db::connect(&p).unwrap().execute("INSERT INTO projects VALUES(1,'Example')",[]).unwrap();let input=serde_json::from_value(json!({"url":"https://example.test/private-reference","projects":[1],"project_only":true})).unwrap();content::capture(&p,input).unwrap();let l=db::load(&p,false).unwrap();println!("AUDIT_PROJECT_LINK: project_only={} source={:?}",l.items[0].project_only,l.items[0].source);assert!(l.items[0].project_only,"Project-only request became a global link");
}
#[test] fn audit_relocation_rebases_thumbnail_history(){
 let t=tempfile::tempdir().unwrap();let old=t.path().join("original");let data=old.join("Library Data");fs::create_dir_all(&data).unwrap();let p=data.join("Library.sqlite");db::init(&p).unwrap();let old=old.canonicalize().unwrap();let c=db::connect(&p).unwrap();db::set_setting(&c,"managed_root",old.to_str().unwrap()).unwrap();let thumb1=data.join("old.png");let thumb2=data.join("new.png");for f in [&thumb1,&thumb2]{image::RgbImage::from_pixel(1,1,image::Rgb([1,2,3])).save(f).unwrap();}let thumb1=thumb1.canonicalize().unwrap();let thumb2=thumb2.canonicalize().unwrap();c.execute("INSERT INTO items(id,path,name,kind,added,thumbnail)VALUES(1,'https://example.test','Fixture','link',1,?)",[thumb2.to_str().unwrap()]).unwrap();history::record(&c,"Replace thumbnail",vec![history::Patch::Thumbnail(1,thumb1.to_string_lossy().into_owned())]).unwrap();drop(c);let moved=t.path().join("moved");fs::rename(&old,&moved).unwrap();let p=moved.join("Library Data/Library.sqlite");managed_library::rebase(&p,&moved).unwrap();edit(&p,json!({"action":"undo_organisation"}));let l=db::load(&p,false).unwrap();println!("AUDIT_RELOCATED_UNDO: thumbnail_exists={}",Path::new(&l.items[0].thumbnail).exists());assert!(Path::new(&l.items[0].thumbnail).exists(),"Undo restored old absolute thumbnail path after library move");
}

#[test] fn audit_unrelated_project_activity_on_undo(){
 let t=tempfile::tempdir().unwrap();let p=t.path().join("db.sqlite");db::init(&p).unwrap();let c=db::connect(&p).unwrap();c.execute_batch("INSERT INTO projects VALUES(1,'First'),(2,'Unrelated');").unwrap();
 edit(&p,json!({"action":"update_studio","value":json!({"id":"client-two","patch":{"kind":"client","name":"Unrelated client","projects":[2]}}).to_string()}));
 edit(&p,json!({"action":"update_client_records","target":2,"value":json!({"type":"record","id":"note","patch":{"kind":"meeting","title":"Unrelated meeting"}}).to_string()}));
 edit(&p,json!({"action":"update_project_overview","target":1,"value":json!({"field":"status","value":"active"}).to_string()}));
 c.execute("DELETE FROM client_events",[]).unwrap();edit(&p,json!({"action":"undo_organisation"}));
 let events:i64=c.query_row("SELECT count(*) FROM client_events WHERE project=2",[],|r|r.get(0)).unwrap();println!("AUDIT_UNRELATED_UNDO: events_for_project_two={events}");assert_eq!(events,0,"Undoing project one created unrelated project two activity");
}

#[test] fn committed_edit_retains_warning_until_repair(){
 let t=tempfile::tempdir().unwrap();let folder=t.path().canonicalize().unwrap();let data=folder.join("Library Data");fs::create_dir(&data).unwrap();let p=data.join("Library.sqlite");db::init(&p).unwrap();db::set_setting(&db::connect(&p).unwrap(),"managed_root",folder.to_str().unwrap()).unwrap();
 let obstruction=folder.join("Browse Library.html");fs::create_dir(&obstruction).unwrap();
 edit(&p,json!({"action":"create_project","name":"Committed project"}));managed_library::after_commit(&p,true);
 let lib=db::load(&p,false).unwrap();assert_eq!(lib.projects.len(),1);assert!(lib.maintenance_warning.is_some());
 fs::remove_dir(&obstruction).unwrap();managed_library::after_commit(&p,true);assert!(db::load(&p,false).unwrap().maintenance_warning.is_none());assert!(obstruction.is_file());
}
#[test] fn download_network_phase_allows_edits_and_cancellation(){
 let t=tempfile::tempdir().unwrap();let p=t.path().join("db.sqlite");db::init(&p).unwrap();let lock=std::sync::Mutex::new(());let cancelled=std::sync::atomic::AtomicBool::new(false);let mut observed=false;
 let result=crate::download::run_locked(&p,&std::env::current_exe().unwrap(),Path::new("unused"),crate::download::Request{url:"https://example.test/video".into(),projects:vec![],tags:vec![]},&cancelled,&mut |progress|{
  if progress.phase=="connecting"{let _guard=lock.try_lock().expect("Download holds edit lock");edit(&p,json!({"action":"create_project","name":"While downloading"}));observed=true;cancelled.store(true,std::sync::atomic::Ordering::SeqCst);}
 },&lock);
 assert!(observed);assert_eq!(result.err().unwrap(),"Download cancelled.");assert_eq!(db::load(&p,false).unwrap().projects.len(),1);assert_eq!(fs::read_dir(t.path().join("video-downloads")).unwrap().count(),0);
}
