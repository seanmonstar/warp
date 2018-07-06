extern crate warp;

#[test]
fn method() {
    let get = warp::get(warp::any());

    let req = warp::test::request();
    assert!(req.matches(&get));


    let req = warp::test::request()
        .method("POST");
    assert!(!req.matches(&get));
}
