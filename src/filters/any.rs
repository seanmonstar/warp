use ::filter::{FilterBase, FilterAnd};

/// A filter that matches any route.
pub fn any() -> Any {
    Any {
        _inner: (),
    }
}

#[derive(Debug)]
pub struct Any {
    _inner: (),
}

impl FilterBase for Any {
    type Extract = ();

    #[inline]
    fn filter(&self) -> Option<Self::Extract> {
        Some(())
    }
}

impl FilterAnd for Any {}

