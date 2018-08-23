#![deny(warnings)]
extern crate pretty_env_logger;
extern crate warp;

use warp::Filter;

#[test]
fn flattens_tuples() {
    let _ = pretty_env_logger::try_init();

    let str1 = warp::any().map(|| "warp");
    let true1 = warp::any().map(|| true);
    let unit1 = warp::any();

    // just 1 value
    let ext = warp::test::request().filter(&str1).unwrap();
    assert_eq!(ext, "warp");

    // just 1 unit
    let ext = warp::test::request().filter(&unit1).unwrap();
    assert_eq!(ext, ());

    // combine 2 values
    let ext = warp::test::request().filter(&str1.and(true1)).unwrap();
    assert_eq!(ext, ("warp", true));

    // combine 2 reversed
    let ext = warp::test::request().filter(&true1.and(str1)).unwrap();
    assert_eq!(ext, (true, "warp"));

    // combine 1 with unit
    let ext = warp::test::request().filter(&str1.and(unit1)).unwrap();
    assert_eq!(ext, "warp");

    let ext = warp::test::request().filter(&unit1.and(str1)).unwrap();
    assert_eq!(ext, "warp");

    // combine 3 values
    let ext = warp::test::request()
        .filter(&str1.and(str1).and(true1))
        .unwrap();
    assert_eq!(ext, ("warp", "warp", true));

    // combine 2 with unit
    let ext = warp::test::request()
        .filter(&str1.and(unit1).and(true1))
        .unwrap();
    assert_eq!(ext, ("warp", true));

    let ext = warp::test::request()
        .filter(&unit1.and(str1).and(true1))
        .unwrap();
    assert_eq!(ext, ("warp", true));

    let ext = warp::test::request()
        .filter(&str1.and(true1).and(unit1))
        .unwrap();
    assert_eq!(ext, ("warp", true));

    // nested tuples
    let str_true_unit = str1.and(true1).and(unit1);
    let unit_str_true = unit1.and(str1).and(true1);

    let ext = warp::test::request()
        .filter(&str_true_unit.and(unit_str_true))
        .unwrap();
    assert_eq!(ext, ("warp", true, "warp", true));

    let ext = warp::test::request()
        .filter(&unit_str_true.and(unit1).and(str1).and(str_true_unit))
        .unwrap();
    assert_eq!(ext, ("warp", true, "warp", "warp", true));
}

#[test]
fn map() {
    let _ = pretty_env_logger::try_init();

    let ok = warp::any().map(warp::reply);

    let req = warp::test::request();
    let resp = req.reply(&ok);
    assert_eq!(resp.status(), 200);
}

#[test]
fn unify() {
    let _ = pretty_env_logger::try_init();

    let a = warp::any().map(|| 1);
    let b = warp::any().map(|| 2);
    let f = a.or(b).unify();

    let ex = warp::test::request().filter(&f).unwrap();
    assert_eq!(ex, 1);
}
