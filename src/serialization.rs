use super::Name;
use serde::{Serialize, Serializer};
use std::borrow::Cow;

#[derive(Serialize)]
struct PrettyNameParts<'a> {
    first_initial: char,
    surname: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    given_name: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    middle_initials: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    middle_names: Option<Cow<'a, str>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    generational_suffix: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    honorific_prefix: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    honorific_suffix: Option<&'a str>,
}

impl Name {
    fn to_pretty_parts(&self) -> PrettyNameParts {
        PrettyNameParts {
            first_initial: self.first_initial(),
            surname: self.surname(),
            given_name: self.given_name(),
            middle_initials: self.middle_initials(),
            middle_names: self.middle_name(),
            generational_suffix: self.generational_suffix(),
            honorific_prefix: self.honorific_prefix(),
            honorific_suffix: self.honorific_suffix(),
        }
    }
}

impl Serialize for Name {
    /// Serializes a name into parsed components.
    ///
    /// ```
    /// use human_name::Name;
    ///
    /// let name = Name::parse("DR JOHN ALLEN Q MACDONALD JR").unwrap();
    /// assert_eq!(
    ///   r#"{"first_initial":"J","surname":"MacDonald","given_name":"John","middle_initials":"AQ","middle_names":"Allen","generational_suffix":"Jr.","honorific_prefix":"Dr."}"#,
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
