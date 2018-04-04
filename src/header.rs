use std::marker::PhantomData;
use std::str::FromStr;

use ::{Filter, Request};

pub fn header<T>(name: &'static str) -> Extract<T> {
    Extract {
        name,
        _marker: PhantomData,
    }
}

pub fn exact(name: &'static str, value: &'static str) -> Exact {
    Exact {
        name,
        value,
    }
}

#[derive(Clone, Debug)]
pub struct Exact {
    name: &'static str,
    value: &'static str,
}

impl Filter for Exact {
    type Extract = ();

    fn filter(&self, input: &mut Request) -> Option<()> {
        trace!("header::Exact({:?}, {:?})", self.name, self.value);
        input.headers()
            .get(self.name)
            .and_then(|val| {
                if val == self.value {
                    Some(())
                } else {
                    None
                }
            })
    }
}

pub struct Extract<T> {
    name: &'static str,
    _marker: PhantomData<T>,
}

impl<T> Filter for Extract<T>
where
    T: FromStr,
{
    type Extract = T;

    fn filter(&self, input: &mut Request) -> Option<T> {
        trace!("header::Extract({:?})", self.name);
        input.headers()
            .get(self.name)
            .and_then(|val| {
                val.to_str().ok()
            })
            .and_then(|s| {
                T::from_str(s).ok()
            })
    }
}
