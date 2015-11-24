use std::hash::{Hash, Hasher};
use super::Name;
use super::comparison;
use super::utils::lowercase_if_alpha;

/// Might this name represent the same person as another name?
///
/// # WARNING
///
/// This is technically an invalid implementation of PartialEq because it is
/// not transitive - "J. Doe" == "Jane Doe", and "J. Doe" == "John Doe", but
/// "Jane Doe" != "John Doe". (It is, however, symmetric and reflexive.)
///
/// Use with caution! See `consistent_with` docs for details.
impl Eq for Name {}
impl PartialEq for Name {
    fn eq(&self, other: &Name) -> bool {
        self.consistent_with(other)
    }
}

/// Implements a hash for a name that is always identical for two names that
/// may be equal.
///
/// # WARNING
///
/// This hash function is prone to collisions!
///
/// We can only use the last four alphabetical characters of the surname, because
/// that's all we're guaranteed to use in the equality test. That means if names
/// are ASCII, we only have 19 bits of variability.
///
/// That means if you are working with a lot of names and you expect surnames
/// to be similar or identical, you might be better off avoiding hash-based
/// datastructures (or using a custom hash and alternate equality test).
///
/// We can't use more characters of the surname because we treat names as equal
/// when one surname ends with the other and the smaller is at least four
/// characters, to catch cases like "Iria Gayo" == "Iria del Río Gayo".
///
/// We can't use the first initial because we might ignore it if someone goes
/// by a middle name, to catch cases like "H. Manuel Alperin" == "Manuel Alperin."
impl Hash for Name {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let surname_chars = self.surnames().iter().flat_map(|w| w.chars()).rev();
        for c in surname_chars.filter_map(lowercase_if_alpha).take(comparison::MIN_SURNAME_CHAR_MATCH) {
            c.hash(state);
        }
    }
}
