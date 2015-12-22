use std::hash::{Hash, Hasher};
use super::Name;

/// Might this name represent the same person as another name?
///
/// ### WARNING
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
/// ### WARNING
///
/// This hash function is prone to collisions!
///
/// See docs on `surname_hash` for details.
///
impl Hash for Name {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.surname_hash(state);
    }
}
