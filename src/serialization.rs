use rustc_serialize::json::{ToJson, Json};
use std::collections::BTreeMap;
use super::Name;

impl ToJson for Name {

    /// Serializes a name into parsed components.
    ///
    /// ```
    /// # extern crate rustc_serialize;
    /// # extern crate human_name;
    /// #
    /// use human_name::Name;
    /// use rustc_serialize::json::ToJson;
    ///
    /// # fn main() {
    /// let name = Name::parse("JOHN ALLEN Q MACDONALD JR").unwrap();
    /// assert_eq!(
    ///   r#"{"first_initial":"J","given_name":"John","middle_initials":"AQ","middle_names":"Allen","suffix":"Jr.","surname":"MacDonald"}"#,
    ///   name.to_json().to_string()
    /// );
    /// # }
    /// ```
    fn to_json(&self) -> Json {
        let mut d = BTreeMap::new();
        d.insert("surname".to_string(), self.surname().to_json());
        d.insert("first_initial".to_string(),
                 format!("{}", self.first_initial()).to_json());
        if self.given_name().is_some() {
            d.insert("given_name".to_string(),
                     self.given_name().unwrap().to_json());
        }
        if self.middle_initials().is_some() {
            d.insert("middle_initials".to_string(),
                     self.middle_initials().unwrap().to_json());
        }
        if self.middle_names().is_some() {
            d.insert("middle_names".to_string(),
                     self.middle_name().unwrap().to_json());
        }
        if self.suffix().is_some() {
            d.insert("suffix".to_string(), self.suffix().unwrap().to_json());
        }
        Json::Object(d)
    }
}
