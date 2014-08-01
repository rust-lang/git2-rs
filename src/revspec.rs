use Object;

pub struct Revspec<'a> {
    from: Option<Object<'a>>,
    to: Option<Object<'a>>,
}

impl<'a> Revspec<'a> {
    pub fn from_objects<'a>(from: Option<Object<'a>>,
                            to: Option<Object<'a>>) -> Revspec<'a> {
        Revspec { from: from, to: to }
    }

    pub fn from(&self) -> Option<&Object<'a>> { self.from.as_ref() }
    pub fn to(&self) -> Option<&Object<'a>> { self.to.as_ref() }
}
