#![deny(warnings)]
use warp::Filter;

#[derive(Clone, Debug, PartialEq)]
struct Ext1(i32);

#[tokio::test]
async fn set_and_get() {
    let ext_filterj = warp::any()
        .with(warp::ext::with_mut(|ext| {
            ext.insert(Ext1(55));
        }))
        .and(warp::ext::get::<Ext1>());

    let extracted = warp::test::request().filter(&ext_filterj).await.unwrap();

    assert_eq!(extracted, Ext1(55));
}

#[tokio::test]
async fn get_missing() {
    let ext = warp::ext::get().map(|e: Ext1| e.0.to_string());

    let res = warp::test::request().reply(&ext).await;

    assert_eq!(res.status(), 500);
    assert_eq!(res.body(), "Missing request extension");
}
