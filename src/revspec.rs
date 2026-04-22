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

    /// Convert a Revspec into the underlying components
    ///
    /// If only references to `from` or `to` are needed, use the corresponding
    /// [`Revspec::from()`] and [`Revspec::to()`] methods.
    pub fn into_objects(self) -> (Option<Object<'repo>>, Option<Object<'repo>>, RevparseMode) {
        (self.from, self.to, self.mode)
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

#[cfg(test)]
mod tests {
    use crate::test::{commit, repo_init};
    use crate::{RevparseMode, Revspec};

    #[test]
    fn round_trip_none() {
        let revspec = Revspec::from_objects(None, None, RevparseMode::SINGLE);
        let objects = revspec.into_objects();
        assert!(objects.0.is_none());
        assert!(objects.1.is_none());
        assert_eq!(RevparseMode::SINGLE, objects.2);

        let revspec = Revspec::from_objects(None, None, RevparseMode::RANGE);
        let objects = revspec.into_objects();
        assert!(objects.0.is_none());
        assert!(objects.1.is_none());
        assert_eq!(RevparseMode::RANGE, objects.2);

        let revspec = Revspec::from_objects(None, None, RevparseMode::MERGE_BASE);
        let objects = revspec.into_objects();
        assert!(objects.0.is_none());
        assert!(objects.1.is_none());
        assert_eq!(RevparseMode::MERGE_BASE, objects.2);
    }

    #[test]
    fn round_trip() {
        let (_td, repo) = repo_init();
        let obj1 = repo.revparse_single("HEAD").unwrap();
        let (id1, kind1) = (obj1.id(), obj1.kind());

        let revspec_just_from = Revspec::from_objects(Some(obj1), None, RevparseMode::SINGLE);
        let objects = revspec_just_from.into_objects();
        assert!(objects.1.is_none());
        assert_eq!(RevparseMode::SINGLE, objects.2);
        let obj1 = objects.0.expect("Should be Some()");
        assert_eq!(id1, obj1.id());
        assert_eq!(kind1, obj1.kind());

        let revspec_just_to = Revspec::from_objects(None, Some(obj1), RevparseMode::SINGLE);
        let objects = revspec_just_to.into_objects();
        assert!(objects.0.is_none());
        assert_eq!(RevparseMode::SINGLE, objects.2);
        let obj1 = objects.1.expect("Should be Some()");
        assert_eq!(id1, obj1.id());
        assert_eq!(kind1, obj1.kind());

        // Need a different object to pass into constructor since it will take
        // ownership; make another commit and ensure that it has a different id
        commit(&repo);
        let obj2 = repo.revparse_single("HEAD").unwrap();
        assert!(obj2.id() != obj1.id());

        let (id2, kind2) = (obj2.id(), obj2.kind());

        let revspec_from_to = Revspec::from_objects(Some(obj1), Some(obj2), RevparseMode::SINGLE);
        let objects = revspec_from_to.into_objects();
        let obj1 = objects.0.expect("Should be Some()");
        assert_eq!(id1, obj1.id());
        assert_eq!(kind1, obj1.kind());
        let obj2 = objects.1.expect("Should be Some()");
        assert_eq!(id2, obj2.id());
        assert_eq!(kind2, obj2.kind());
        assert_eq!(RevparseMode::SINGLE, objects.2);
    }
}
