//! Cookie Filters

use http::header::HeaderValue;

use ::never::Never;
use ::filter::{Filter, One};
use ::reject::Rejection;
use super::header;

/// Creates a `Filter` that requires a cookie by name.
///
/// If found, extracts the value of the cookie, otherwise rejects.
pub fn cookie(name: &'static str) -> impl Filter<Extract=One<String>, Error=Rejection> + Copy {
    header::value(&::http::header::COOKIE, move |val| {
        find_cookie(name, val)
    })
}

/// Creates a `Filter` that looks for an optional cookie by name.
///
/// If found, extracts the value of the cookie, otherwise continues
/// the request, extracting `None`.
pub fn optional(name: &'static str) -> impl Filter<Extract=One<Option<String>>, Error=Never> + Copy {
    header::optional_value(&::http::header::COOKIE, move |val| {
        find_cookie(name, val)
    })
}

/// Creates a `Filter` that looks for an optional cookie by name and maps the value.
///
/// If found, extracts the value of the cookie and applies the provided function to it,
/// otherwise continues the request, extracting `None`.
pub fn optional_value<U, F>(name: &'static str, func: F)
    -> impl Filter<Extract=One<Option<U>>, Error=Never> + Copy
where
    F: Fn(&str) -> U + Copy,
    U: Send,
{
    header::optional_value(&::http::header::COOKIE, move |val| {
        find_cookie(name, val).map(|v| func(v.as_ref()))
    })
}

//TODO: probably shouldn't extract a `String`, but rather a `Cookie`.
//That would allow use to change from cloning a `String` to just shallow cloning
//the `Bytes` of the header value...
fn find_cookie(name: &str, value: &HeaderValue) -> Option<String> {
    value
        .to_str()
        .ok()
        .and_then(|value| {
            //TODO: could be optimized, and there's edge cases not handled...
            for pair in value.split(';') {
                let pair = pair.trim();
                // name.len() + `=`
                if pair.len() > name.len() + 1 {
                    if pair.starts_with(name) {
                        if pair.as_bytes()[name.len()] == b'=' {
                            return Some(pair[name.len() + 1..].to_string())
                        }
                    }
                }
            }

            None
        })
}
