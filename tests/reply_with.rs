extern crate pretty_env_logger;
extern crate warp;

use warp::Filter;

#[test]
fn header() {
    let header = warp::reply::with::header("foo", "bar");

    let no_header = header.decorate(
        warp::any().map(warp::reply)
    );

    let req = warp::test::request();
    let resp = req.reply(&no_header);
    assert_eq!(resp.headers()["foo"], "bar");

    let prev_header = warp::reply::with::header("foo", "sean");
    let yes_header = header.decorate(
        prev_header.decorate(warp::any().map(warp::reply))
    );

    let req = warp::test::request();
    let resp = req.reply(&yes_header);
    assert_eq!(resp.headers()["foo"], "bar", "replaces header");
}

#[test]
fn default_header() {
    let def_header = warp::reply::with::default_header("foo", "bar");

    let no_header = def_header.decorate(
        warp::any().map(warp::reply)
    );

    let req = warp::test::request();
    let resp = req.reply(&no_header);

    assert_eq!(resp.headers()["foo"], "bar");

    let header = warp::reply::with::header("foo", "sean");
    let yes_header = def_header.decorate(
        header.decorate(warp::any().map(warp::reply))
    );

    let req = warp::test::request();
    let resp = req.reply(&yes_header);

    assert_eq!(resp.headers()["foo"], "sean", "doesn't replace header");
}
