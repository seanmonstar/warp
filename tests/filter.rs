#![deny(warnings)]
extern crate pretty_env_logger;
extern crate warp;
extern crate hyper;
extern crate tower_service;
extern crate futures;

use warp::Filter;

#[test]
fn flattens_tuples() {
    let _ = pretty_env_logger::try_init();

    let str1 = warp::any().map(|| "warp");
    let true1 = warp::any().map(|| true);
    let unit1 = warp::any();

    // just 1 value
    let ext = warp::test::request()
        .filter(&str1)
        .unwrap();
    assert_eq!(ext, "warp");

    // just 1 unit
    let ext = warp::test::request()
        .filter(&unit1)
        .unwrap();
    assert_eq!(ext, ());

    // combine 2 values
    let ext = warp::test::request()
        .filter(&str1.and(true1))
        .unwrap();
    assert_eq!(ext, ("warp", true));

    // combine 2 reversed
    let ext = warp::test::request()
        .filter(&true1.and(str1))
        .unwrap();
    assert_eq!(ext, (true, "warp"));

    // combine 1 with unit
    let ext = warp::test::request()
        .filter(&str1.and(unit1))
        .unwrap();
    assert_eq!(ext, "warp");

    let ext = warp::test::request()
        .filter(&unit1.and(str1))
        .unwrap();
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

    let ex = warp::test::request()
        .filter(&f)
        .unwrap();
    assert_eq!(ex, 1);
}

#[test]
fn lift() {
    let it = warp::any().map(|| 1);
    let _lift = it.lift();
    let _lift = ::std::rc::Rc::new(it).lift();
    let _lift = ::std::sync::Arc::new(it).lift();
}

#[test]
fn lift_to_tower_service() {
    use tower_service::Service;
    use futures::{Future, Async};
    use hyper::{Request, Body, StatusCode};

    let _ = pretty_env_logger::try_init();

    // 200 OK
    {
        let mut it = warp::any().map(|| "ok").lift();
        let req = Request::new(Body::empty());
        match it.call(req).poll() {
            Ok(Async::Ready(res)) => assert_eq!(200, res.status()),
            err => unreachable!("{:?}", err)
        }
    }

    // 500 Server Error
    {
        let mut it = warp::any()
          .and_then(|| {
              Err::<StatusCode, _>(warp::reject::server_error())
          })
          .lift();
        let req = Request::new(Body::empty());
        match it.call(req).poll() {
            Ok(Async::Ready(res)) => assert_eq!(500, res.status()),
            err => unreachable!("{:?}", err)
        }
    }
}

#[test]
fn lift_to_hyper_service() {
    use futures::{Future, Async};
    use hyper::{Request, Body, StatusCode, service::Service};

    let _ = pretty_env_logger::try_init();

    // 200 OK
    {
        let mut it = warp::any().map(|| "ok").lift();
        let req = Request::new(Body::empty());
        match it.call(req).poll() {
            Ok(Async::Ready(res)) => assert_eq!(200, res.status()),
            err => unreachable!("{:?}", err)
        }
    }

    // 500 Server Error
    {
        let mut it = warp::any()
          .and_then(|| {
              Err::<StatusCode, _>(warp::reject::server_error())
          })
          .lift();
        let req = Request::new(Body::empty());
        match it.call(req).poll() {
            Ok(Async::Ready(res)) => assert_eq!(500, res.status()),
            err => unreachable!("{:?}", err)
        }
    }
}
