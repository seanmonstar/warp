extern crate pretty_env_logger;
extern crate warp;

use warp::Filter;

#[test]
fn map() {
    let _ = pretty_env_logger::try_init();

    let ok = warp::any().map(warp::reply);

    let req = warp::test::request();
    let resp = req.reply(&ok);
    assert_eq!(resp.status(), 200);
}
