#![deny(warnings)]
use warp::Filter;

#[tokio::test]
async fn none_is_not_found() {
    let filter = warp::none();

    // none should return not found for request
    let req = warp::test::request();
    assert!(req.filter(&filter).await.unwrap_err().is_not_found());
}

#[tokio::test]
async fn none_can_be_chained() {
    let req = warp::test::request();
    let filter = warp::none();

    // should not match anything
    assert!(!req.matches(&filter).await);

    let req = warp::test::request();
    let filter = filter.or(warp::get().map(warp::reply));

    // this should now match because we chained the get with 'or'
    assert!(req.matches(&filter).await);
}
