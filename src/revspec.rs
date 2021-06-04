use crate::{Object, RevparseMode};

/// A revspec represents a range of revisions within a repository.
pub struct Revspec<'repo> {
    from: Option<Object<'repo>>,
    to: Option<Object<'repo>>,
    mode: RevparseMode,
}

impl<'repo> Revspec<'repo> {
    /// Assembles a new revspec from the from/to components.
    pub fn from_objects(
        from: Option<Object<'repo>>,
        to: Option<Object<'repo>>,
        mode: RevparseMode,
    ) -> Revspec<'repo> {
        Revspec { from, to, mode }
    }

    /// Access the `from` range of this revspec.
    pub fn from(&self) -> Option<&Object<'repo>> {
        self.from.as_ref()
    }

    /// Access the `to` range of this revspec.
    pub fn to(&self) -> Option<&Object<'repo>> {
        self.to.as_ref()
    }

    /// Returns the intent of the revspec.
    pub fn mode(&self) -> RevparseMode {
        self.mode
    }
}
