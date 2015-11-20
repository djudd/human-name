use rustc_serialize::json::{ToJson, Json};
use std::collections::BTreeMap;
use super::Name;

impl ToJson for Name {
    fn to_json(&self) -> Json {
        let mut d = BTreeMap::new();
        d.insert("surname".to_string(), self.surname().to_json());
        d.insert("first_initial".to_string(), format!("{}", self.first_initial()).to_json());
        if self.given_name().is_some() {
            d.insert("given_name".to_string(), self.given_name().unwrap().to_json());
        }
        if self.middle_initials().is_some() {
            d.insert("middle_initial".to_string(), self.middle_initials().unwrap().to_json());
        }
        if self.middle_names().is_some() {
            d.insert("middle_names".to_string(), self.middle_name().unwrap().to_json());
        }
        if self.suffix().is_some() {
            d.insert("suffix".to_string(), self.suffix().unwrap().to_json());
        }
        Json::Object(d)
    }
}
