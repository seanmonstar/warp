use super::*;
use http::StatusCode;

#[derive(Debug, PartialEq)]
struct Left;

#[derive(Debug, PartialEq)]
struct Right;

impl Reject for Left {}
impl Reject for Right {}

#[test]
fn rejection_status() {
    assert_eq!(not_found().status(), StatusCode::NOT_FOUND);
    assert_eq!(
        method_not_allowed().status(),
        StatusCode::METHOD_NOT_ALLOWED
    );
    assert_eq!(length_required().status(), StatusCode::LENGTH_REQUIRED);
    assert_eq!(payload_too_large().status(), StatusCode::PAYLOAD_TOO_LARGE);
    assert_eq!(
        unsupported_media_type().status(),
        StatusCode::UNSUPPORTED_MEDIA_TYPE
    );
    assert_eq!(custom(Left).status(), StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test]
async fn combine_rejection_causes_with_some_left_and_none_right() {
    let left = custom(Left);
    let right = not_found();
    let reject = left.combine(right);
    let resp = reject.into_response();

    assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(
        response_body_string(resp).await,
        "Unhandled rejection: Left"
    )
}

#[tokio::test]
async fn combine_rejection_causes_with_none_left_and_some_right() {
    let left = not_found();
    let right = custom(Right);
    let reject = left.combine(right);
    let resp = reject.into_response();

    assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(
        response_body_string(resp).await,
        "Unhandled rejection: Right"
    )
}

#[tokio::test]
async fn unhandled_customs() {
    let reject = not_found().combine(custom(Right));

    let resp = reject.into_response();
    assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(
        response_body_string(resp).await,
        "Unhandled rejection: Right"
    );

    // There's no real way to determine which is worse, since both are a 500,
    // so pick the first one.
    let reject = custom(Left).combine(custom(Right));

    let resp = reject.into_response();
    assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(
        response_body_string(resp).await,
        "Unhandled rejection: Left"
    );

    // With many rejections, custom still is top priority.
    let reject = not_found()
        .combine(not_found())
        .combine(not_found())
        .combine(custom(Right))
        .combine(not_found());

    let resp = reject.into_response();
    assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(
        response_body_string(resp).await,
        "Unhandled rejection: Right"
    );
}

async fn response_body_string(resp: crate::reply::Response) -> String {
    let (_, body) = resp.into_parts();
    let body_bytes = hyper::body::to_bytes(body).await.expect("failed concat");
    String::from_utf8_lossy(&body_bytes).to_string()
}

#[test]
fn find_cause() {
    let rej = custom(Left);

    assert_eq!(rej.find::<Left>(), Some(&Left));

    let rej = rej.combine(method_not_allowed());

    assert_eq!(rej.find::<Left>(), Some(&Left));
    assert!(rej.find::<MethodNotAllowed>().is_some(), "MethodNotAllowed");
}

#[test]
fn size_of_rejection() {
    assert_eq!(
        ::std::mem::size_of::<Rejection>(),
        ::std::mem::size_of::<usize>() * 4,
    );
}

#[test]
fn basic_fatal_creation() {
    let err = "test";
    assert_eq!(
        format!("{:?}", fatal(err).into_response().body()),
        format!("{:?}", err.into_response().body())
    );
    assert_eq!(fatal(err).status(), StatusCode::OK);
}

#[derive(Debug)]
struct X(u32);
impl Reject for X {}

fn combine_n<F, R>(n: u32, new_reject: F) -> Rejection
where
    F: Fn(u32) -> R,
    R: Reject,
{
    let mut rej = not_found();

    for i in 0..n {
        rej = rej.combine(custom(new_reject(i)));
    }

    rej
}

#[test]
fn test_debug() {
    let rej = combine_n(3, X);

    let s = format!("{:?}", rej);
    assert_eq!(s, "Rejection { reason: Mismatch([Custom(X(0)), Custom(X(1)), Custom(X(2))]) }");
}
