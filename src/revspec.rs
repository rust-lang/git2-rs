use Object;

pub struct Revspec {
    from: Option<Object>,
    to: Option<Object>,
}

impl Revspec {
    pub fn from_objects(from: Option<Object>, to: Option<Object>) -> Revspec {
        Revspec { from: from, to: to }
    }

    pub fn from(&self) -> Option<&Object> { self.from.as_ref() }
    pub fn to(&self) -> Option<&Object> { self.to.as_ref() }
}
