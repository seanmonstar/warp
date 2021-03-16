#![deny(warnings)]
use warp::Filter;

#[tokio::test]
async fn no_authorization() {
    let auth = warp::auth::basic().allow("user", "1234");
    let route = warp::any().map(warp::reply).with(auth);
    let res = warp::test::request().method("GET").reply(&route).await;

    assert_eq!(res.status(), 401);
    assert_eq!(res.headers()["www-authenticate"], "Basic charset=\"UTF-8\"");
}

#[tokio::test]
async fn wrong_authorization() {
    let auth = warp::auth::basic().allow("user", "1234");
    let route = warp::any().map(warp::reply).with(auth);
    let res = warp::test::request()
        .method("GET")
        .header("authorization", "Basic YWRtaW46NDMyMQ==")
        .reply(&route)
        .await;

    assert_eq!(res.status(), 401);
    assert_eq!(res.headers()["www-authenticate"], "Basic charset=\"UTF-8\"");
}

#[tokio::test]
async fn right_authorization() {
    let auth = warp::auth::basic().allow("user", "1234");
    let route = warp::any().map(warp::reply).with(auth);
    let res = warp::test::request()
        .method("GET")
        .header("authorization", "Basic dXNlcjoxMjM0")
        .reply(&route)
        .await;

    assert_eq!(res.status(), 200);
    assert_eq!(res.headers().get("www-authenticate"), None);
}

#[tokio::test]
async fn many_authorization() {
    let auth = warp::auth::basic()
        .allow("user", "1234")
        .allow("admin", "4321");
    let route = warp::any().map(warp::reply).with(auth);
    let res_user = warp::test::request()
        .method("GET")
        .header("authorization", "Basic dXNlcjoxMjM0")
        .reply(&route)
        .await;
    let res_admin = warp::test::request()
        .method("GET")
        .header("authorization", "Basic YWRtaW46NDMyMQ==")
        .reply(&route)
        .await;
    let res_other = warp::test::request()
        .method("GET")
        .header("authorization", "Basic d3Jvbmc6MTIzNA==")
        .reply(&route)
        .await;

    assert_eq!(res_user.status(), 200);
    assert_eq!(res_user.headers().get("www-authenticate"), None);
    assert_eq!(res_admin.status(), 200);
    assert_eq!(res_admin.headers().get("www-authenticate"), None);
    assert_eq!(res_other.status(), 401);
    assert_eq!(
        res_other.headers()["www-authenticate"],
        "Basic charset=\"UTF-8\""
    );
}

#[tokio::test]
async fn realm_authorization() {
    let auth = warp::auth::basic().realm("whatever").allow("user", "1234");
    let route = warp::any().map(warp::reply).with(auth);
    let res = warp::test::request().method("GET").reply(&route).await;

    assert_eq!(res.status(), 401);
    assert_eq!(
        res.headers()["www-authenticate"],
        "Basic realm=\"whatever\", charset=\"UTF-8\""
    );
}
