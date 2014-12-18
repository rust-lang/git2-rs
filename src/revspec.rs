use Object;

/// A revspec represents a range of revisions within a repository.
pub struct Revspec<'repo> {
    from: Option<Object<'repo>>,
    to: Option<Object<'repo>>,
}

impl<'repo> Revspec<'repo> {
    /// Assembles a new revspec from the from/to components.
    pub fn from_objects(from: Option<Object<'repo>>,
                        to: Option<Object<'repo>>) -> Revspec<'repo> {
        Revspec { from: from, to: to }
    }

    /// Access the `from` range of this revspec.
    pub fn from(&self) -> Option<&Object<'repo>> { self.from.as_ref() }

    /// Access the `to` range of this revspec.
    pub fn to(&self) -> Option<&Object<'repo>> { self.to.as_ref() }
}
