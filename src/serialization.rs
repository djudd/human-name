use super::Name;
use rustc_serialize::json::{Json, ToJson};
use std::collections::BTreeMap;

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
        d.insert(
            "first_initial".to_string(),
            format!("{}", self.first_initial()).to_json(),
        );
        if let Some(name) = self.given_name() {
            d.insert("given_name".to_string(), name.to_json());
        }
        if let Some(initials) = self.middle_initials() {
            d.insert("middle_initials".to_string(), initials.to_json());
        }
        if let Some(name) = self.middle_name() {
            d.insert("middle_names".to_string(), name.to_json());
        }
        if let Some(suffix) = self.suffix() {
            d.insert("suffix".to_string(), suffix.to_json());
        }
        Json::Object(d)
    }
}
