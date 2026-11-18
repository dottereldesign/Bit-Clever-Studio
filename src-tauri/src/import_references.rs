//! Backup IDs belong to the source library, never the destination. Missing live
//! references are dropped; activity retains an explicit unavailable marker.
use std::collections::HashMap;
use serde_json::Value;
use crate::db::Result;

pub struct ImportReferences<'a> {
    projects: &'a HashMap<i64, i64>,
    items: &'a HashMap<i64, i64>,
}
impl<'a> ImportReferences<'a> {
    pub fn new(projects: &'a HashMap<i64, i64>, items: &'a HashMap<i64, i64>) -> Self { Self { projects, items } }
    pub fn project(&self, id: Option<i64>) -> Option<i64> { id.and_then(|id| self.projects.get(&id).copied()) }
    pub fn item(&self, id: Option<i64>) -> Option<i64> { id.and_then(|id| self.items.get(&id).copied()) }
    pub fn required_project(&self, id: i64, message: &str) -> Result<i64> { self.project(Some(id)).ok_or_else(|| message.to_owned()) }
    pub fn projects(&self, ids: &[i64]) -> Vec<i64> { ids.iter().filter_map(|id| self.project(Some(*id))).collect() }
    pub fn items(&self, ids: &[i64]) -> Vec<i64> { ids.iter().filter_map(|id| self.item(Some(*id))).collect() }
    pub fn activity(&self, value: &mut Value) {
        for (key, map) in [("files", self.items), ("projects", self.projects)] {
            if let Some(ids) = value.get_mut(key).and_then(Value::as_array_mut) {
                for id in ids {
                    if let Some(old) = id.as_i64() {
                        *id = map.get(&old).map(|id| Value::from(*id)).unwrap_or_else(|| Value::from("Unavailable historical reference"));
                    }
                }
            }
        }
        if let Some(id) = value.get("project_id").and_then(Value::as_i64) {
            value["project_id"] = self.project(Some(id)).map(Value::from).unwrap_or(Value::Null);
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn source_ids_never_fall_back_to_destination_ids() {
        let projects = HashMap::from([(1, 101)]);
        let items = HashMap::from([(1, 201)]);
        let refs = ImportReferences::new(&projects, &items);
        assert_eq!(refs.projects(&[1, 101, 1]), vec![101, 101]);
        assert_eq!(refs.items(&[1, 201]), vec![201]);
        assert_eq!(refs.item(None), None);
        assert!(refs.required_project(101, "Missing owner").is_err());
        let mut activity = serde_json::json!({"files":[1,201,"existing marker"],"projects":[1,101],"project_id":101,"title":"Unchanged"});
        refs.activity(&mut activity);
        assert_eq!(activity, serde_json::json!({"files":[201,"Unavailable historical reference","existing marker"],"projects":[101,"Unavailable historical reference"],"project_id":null,"title":"Unchanged"}));
    }
}
