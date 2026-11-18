//! Opt-in synthetic profiling. Never opens the user's library or media.
use crate::db;
use std::time::Instant;

#[test]
#[ignore = "Synthetic 4k/20k profiling; run explicitly with --ignored --nocapture"]
fn synthetic_library_profile() {
    for count in [4_000, 20_000] {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("profile.sqlite");
        db::init(&path).unwrap();
        {
            let mut connection = db::connect(&path).unwrap();
            let tx = connection.transaction().unwrap();
            for id in 1001..=1040 {
                tx.execute("INSERT INTO projects(id,name) VALUES(?1,?2)", rusqlite::params![id,format!("Project {id}")]).unwrap();
                tx.execute("INSERT INTO tags(id,name) VALUES(?1,?2)", rusqlite::params![id,format!("Tag {id}")]).unwrap();
            }
            for id in 1..=count {
                tx.execute("INSERT INTO items(id,path,name,kind,source,added) VALUES(?1,?2,?3,'image','Design',1)", rusqlite::params![id,format!("C:/synthetic/{id}.png"),format!("Reference {id}")]).unwrap();
                for label in [id%40+1001,(id+1)%40+1001] {
                    tx.execute("INSERT INTO item_projects(item_id,label_id) VALUES(?1,?2)", [id,label]).unwrap();
                    tx.execute("INSERT INTO item_tags(item_id,label_id) VALUES(?1,?2)", [id,label]).unwrap();
                }
                tx.execute("INSERT INTO item_categories(item_id,name) VALUES(?1,'Poster')", [id]).unwrap();
                tx.execute("INSERT INTO category_arrivals(item_id,name,added) VALUES(?1,'Poster',1)", [id]).unwrap();
            }
            tx.commit().unwrap();
        }
        // First read has a new SQLite connection but OS pages remain warm after seeding.
        // Each db::load opens a new connection; later reads are not a retained SQLite cache.
        for repetition in 0..4 {
            let start=Instant::now();
            let library=db::load(&path,false).unwrap();
            let read_ms=start.elapsed().as_secs_f64()*1000.;
            assert_eq!(library.items.len(),count as usize);
            assert!(library.items.iter().all(|i|i.projects.len()==2 && i.tags.len()==2));
            let start=Instant::now();
            let json=serde_json::to_vec(&library).unwrap();
            let serialize_ms=start.elapsed().as_secs_f64()*1000.;
            println!("PROFILE {}",serde_json::json!({"items":count,"repetition":repetition,"read_ms":read_ms,"serialize_ms":serialize_ms,"json_bytes":json.len(),"debug":cfg!(debug_assertions)}));
        }
        let mut writer=db::connect(&path).unwrap();
        let tx=writer.transaction().unwrap();
        tx.execute("UPDATE items SET name='uncommitted' WHERE id=1",[]).unwrap();
        let start=Instant::now();
        let library=db::load(&path,false).unwrap();
        let elapsed=start.elapsed().as_secs_f64()*1000.;
        assert_eq!(library.items[0].name,"Reference 1");
        println!("PROFILE {}",serde_json::json!({"items":count,"read_during_open_wal_writer_ms":elapsed}));
        tx.rollback().unwrap();
    }
}
