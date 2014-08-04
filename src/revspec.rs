use Object;

/// A revspec represents a range of revisions within a repository.
pub struct Revspec<'a> {
    from: Option<Object<'a>>,
    to: Option<Object<'a>>,
}

impl<'a> Revspec<'a> {
    /// Assembles a new revspec from the from/to components.
    pub fn from_objects<'a>(from: Option<Object<'a>>,
                            to: Option<Object<'a>>) -> Revspec<'a> {
        Revspec { from: from, to: to }
    }

    /// Access the `from` range of this revspec.
    pub fn from(&self) -> Option<&Object<'a>> { self.from.as_ref() }

    /// Access the `to` range of this revspec.
    pub fn to(&self) -> Option<&Object<'a>> { self.to.as_ref() }
}
