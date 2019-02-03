use super::Name;
use serde::ser::{Serialize, Serializer};
use std::borrow::Cow;

#[derive(Serialize)]
struct PrettyNameParts<'a> {
    first_initial: char,
    surname: &'a str,
    given_name: Option<&'a str>,
    middle_initials: Option<&'a str>,
    middle_names: Option<Cow<'a, str>>,
    suffix: Option<&'a str>,
}

impl Name {
    fn to_pretty_parts(&self) -> PrettyNameParts {
        PrettyNameParts {
            first_initial: self.first_initial(),
            surname: self.surname(),
            given_name: self.given_name(),
            middle_initials: self.middle_initials(),
            middle_names: self.middle_name(),
            suffix: self.suffix(),
        }
    }
}

impl Serialize for Name {
    /// Serializes a name into parsed components.
    ///
    /// ```
    /// use human_name::Name;
    ///
    /// let name = Name::parse("JOHN ALLEN Q MACDONALD JR").unwrap();
    /// assert_eq!(
    ///   r#"{"first_initial":"J","surname":"MacDonald","given_name":"John","middle_initials":"AQ","middle_names":"Allen","suffix":"Jr."}"#,
    ///   serde_json::to_string(&name).unwrap()
    /// );
    /// ```
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.to_pretty_parts().serialize(serializer)
    }
}
