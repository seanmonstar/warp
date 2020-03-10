#![deny(warnings)]

#[macro_use]
extern crate serde_derive;

use bytes::Buf;
use futures::TryStreamExt;
use warp::Filter;

#[tokio::test]
async fn matches() {
    let _ = pretty_env_logger::try_init();

    let concat = warp::body::bytes();

    let req = warp::test::request().path("/nothing-matches-me");

    assert!(req.matches(&concat).await);

    let p = warp::path("body");
    let req = warp::test::request().path("/body");

    let and = p.and(concat);

    assert!(req.matches(&and).await);
}

#[tokio::test]
async fn server_error_if_taking_body_multiple_times() {
    let _ = pretty_env_logger::try_init();

    let concat = warp::body::bytes();
    let double = concat.and(concat).map(|_, _| warp::reply());

    let res = warp::test::request().reply(&double).await;

    assert_eq!(res.status(), 500);
    assert_eq!(res.body(), "Request body consumed multiple times");
}

#[tokio::test]
async fn content_length_limit() {
    let _ = pretty_env_logger::try_init();

    let limit = warp::body::content_length_limit(30).map(warp::reply);

    let res = warp::test::request().reply(&limit).await;
    assert_eq!(res.status(), 411, "missing content-length returns 411");

    let res = warp::test::request()
        .header("content-length", "999")
        .reply(&limit)
        .await;
    assert_eq!(res.status(), 413, "over limit returns 413");

    let res = warp::test::request()
        .header("content-length", "2")
        .reply(&limit)
        .await;
    assert_eq!(res.status(), 200, "under limit succeeds");
}

#[tokio::test]
async fn json() {
    let _ = pretty_env_logger::try_init();

    let json = warp::body::json::<Vec<i32>>();

    let req = warp::test::request().body("[1, 2, 3]");

    let vec = req.filter(&json).await.unwrap();
    assert_eq!(vec, &[1, 2, 3]);

    let req = warp::test::request()
        .header("content-type", "application/json")
        .body("[3, 2, 1]");

    let vec = req.filter(&json).await.unwrap();
    assert_eq!(vec, &[3, 2, 1], "matches content-type");
}

#[tokio::test]
async fn json_rejects_bad_content_type() {
    let _ = pretty_env_logger::try_init();

    let json = warp::body::json::<Vec<i32>>().map(|_| warp::reply());

    let req = warp::test::request()
        .header("content-type", "text/xml")
        .body("[3, 2, 1]");

    let res = req.reply(&json).await;
    assert_eq!(
        res.status(),
        415,
        "bad content-type should be 415 Unsupported Media Type"
    );
}

#[tokio::test]
async fn json_invalid() {
    let _ = pretty_env_logger::try_init();

    let json = warp::body::json::<Vec<i32>>().map(|vec| warp::reply::json(&vec));

    let res = warp::test::request().body("lol#wat").reply(&json).await;
    assert_eq!(res.status(), 400);
    let prefix = b"Request body deserialize error: ";
    assert_eq!(&res.body()[..prefix.len()], prefix);
}

#[test]
fn json_size_of() {
    let json = warp::body::json_enforce_strict_content_type::<Vec<i32>>();
    assert_eq!(std::mem::size_of_val(&json), 1);
}

#[tokio::test]
async fn json_enforce_strict_content_type() {
    let _ = pretty_env_logger::try_init();

    let json = warp::body::json_enforce_strict_content_type::<Vec<i32>>().map(|_| warp::reply());

    let req = warp::test::request().body("[1, 2, 3]");

    let res = req.reply(&json).await;
    assert_eq!(
        res.status(),
        415,
        "bad content-type should be 415 Unsupported Media Type"
    );

    let json = warp::body::json_enforce_strict_content_type::<Vec<i32>>();

    let req = warp::test::request()
        .header("content-type", "application/json")
        .body("[3, 2, 1]");

    let vec = req.filter(&json).await.unwrap();
    assert_eq!(vec, &[3, 2, 1], "matches content-type");
}

#[tokio::test]
async fn json_enforce_strict_content_type_rejects_bad_content_type() {
    let _ = pretty_env_logger::try_init();

    let json = warp::body::json_enforce_strict_content_type::<Vec<i32>>().map(|_| warp::reply());

    let req = warp::test::request()
        .header("content-type", "text/xml")
        .body("[3, 2, 1]");

    let res = req.reply(&json).await;
    assert_eq!(
        res.status(),
        415,
        "bad content-type should be 415 Unsupported Media Type"
    );
}

#[tokio::test]
async fn json_enforce_strict_content_type_invalid() {
    let _ = pretty_env_logger::try_init();

    let json = warp::body::json_enforce_strict_content_type::<Vec<i32>>()
        .map(|vec| warp::reply::json(&vec));

    let res = warp::test::request()
        .body("lol#wat")
        .header("content-type", "application/json")
        .reply(&json)
        .await;
    assert_eq!(res.status(), 400);
    let prefix = b"Request body deserialize error: ";
    assert_eq!(&res.body()[..prefix.len()], prefix);
}

#[test]
fn json_enforce_strict_content_type_size_of() {
    let json = warp::body::json_enforce_strict_content_type::<Vec<i32>>();
    assert_eq!(std::mem::size_of_val(&json), 1);
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct Item {
    name: String,
    source: String,
}

#[tokio::test]
async fn xml() {
    let _ = pretty_env_logger::try_init();

    let xml = warp::body::xml::<Item>();

    let req = warp::test::request().body("<item><name>Warp</name><source>GitHub</source></item>");

    let item = req.filter(&xml).await.unwrap();
    assert_eq!(
        item,
        Item {
            name: "Warp".to_string(),
            source: "GitHub".to_string()
        },
    );

    let req = warp::test::request()
        .header("content-type", "text/xml")
        .body("<item><name>Warp</name><source>GitHub</source></item>");

    let item = req.filter(&xml).await.unwrap();
    assert_eq!(
        item,
        Item {
            name: "Warp".to_string(),
            source: "GitHub".to_string()
        },
        "matches content-type text/xml"
    );

    let req = warp::test::request()
        .header("content-type", "application/xml")
        .body("<item><name>Warp</name><source>GitHub</source></item>");

    let item = req.filter(&xml).await.unwrap();
    assert_eq!(
        item,
        Item {
            name: "Warp".to_string(),
            source: "GitHub".to_string()
        },
        "matches content-type application/xml"
    );
}

#[tokio::test]
async fn xml_rejects_bad_content_type() {
    let _ = pretty_env_logger::try_init();

    let xml = warp::body::xml::<Item>().map(|_| warp::reply());

    let req = warp::test::request()
        .header("content-type", "application/json")
        .body("<item><name>Warp</name><source>GitHub</source></item>");

    let res = req.reply(&xml).await;
    assert_eq!(
        res.status(),
        415,
        "bad content-type should be 415 Unsupported Media Type"
    );
}

#[tokio::test]
async fn xml_invalid() {
    let _ = pretty_env_logger::try_init();

    let xml = warp::body::xml::<Item>().map(|item| warp::reply::json(&item));

    let res = warp::test::request().body("lol#wat").reply(&xml).await;
    assert_eq!(res.status(), 400);
    let prefix = b"Request body deserialize error: ";
    assert_eq!(&res.body()[..prefix.len()], prefix);
}

#[tokio::test]
async fn xml_enforce_strict_content_type() {
    let _ = pretty_env_logger::try_init();

    let xml = warp::body::xml_enforce_strict_content_type::<Item>().map(|_| warp::reply());

    let req = warp::test::request().body("<item><name>Warp</name><source>GitHub</source></item>");
    let res = req.reply(&xml).await;

    assert_eq!(
        res.status(),
        415,
        "bad content-type should be 415 Unsupported Media Type"
    );

    let xml = warp::body::xml_enforce_strict_content_type::<Item>();

    let req = warp::test::request()
        .header("content-type", "text/xml")
        .body("<item><name>Warp</name><source>GitHub</source></item>");

    let item = req.filter(&xml).await.unwrap();
    assert_eq!(
        item,
        Item {
            name: "Warp".to_string(),
            source: "GitHub".to_string()
        },
        "matches content-type text/xml"
    );

    let req = warp::test::request()
        .header("content-type", "application/xml")
        .body("<item><name>Warp</name><source>GitHub</source></item>");

    let item = req.filter(&xml).await.unwrap();
    assert_eq!(
        item,
        Item {
            name: "Warp".to_string(),
            source: "GitHub".to_string()
        },
        "matches content-type application/xml"
    );
}

#[tokio::test]
async fn xml_enforce_strict_content_type_rejects_bad_content_type() {
    let _ = pretty_env_logger::try_init();

    let xml = warp::body::xml_enforce_strict_content_type::<Item>().map(|_| warp::reply());

    let req = warp::test::request()
        .header("content-type", "application/json")
        .body("<item><name>Warp</name><source>GitHub</source></item>");

    let res = req.reply(&xml).await;
    assert_eq!(
        res.status(),
        415,
        "bad content-type should be 415 Unsupported Media Type"
    );
}

#[tokio::test]
async fn xml_enforce_strict_content_type_invalid() {
    let _ = pretty_env_logger::try_init();

    let xml =
        warp::body::xml_enforce_strict_content_type::<Item>().map(|item| warp::reply::json(&item));

    let res = warp::test::request()
        .body("lol#wat")
        .header("content-type", "application/xml")
        .reply(&xml)
        .await;
    assert_eq!(res.status(), 400);
    let prefix = b"Request body deserialize error: ";
    assert_eq!(&res.body()[..prefix.len()], prefix);
}

#[tokio::test]
async fn form() {
    let _ = pretty_env_logger::try_init();

    let form = warp::body::form::<Vec<(String, String)>>();

    let req = warp::test::request().body("foo=bar&baz=quux");

    let vec = req.filter(&form).await.unwrap();
    let expected = vec![
        ("foo".to_owned(), "bar".to_owned()),
        ("baz".to_owned(), "quux".to_owned()),
    ];
    assert_eq!(vec, expected);
}

#[tokio::test]
async fn form_rejects_bad_content_type() {
    let _ = pretty_env_logger::try_init();

    let form = warp::body::form::<Vec<(String, String)>>().map(|_| warp::reply());

    let req = warp::test::request()
        .header("content-type", "application/x-www-form-urlencoded")
        .body("foo=bar");

    let res = req.reply(&form).await;
    assert_eq!(res.status(), 200);

    let req = warp::test::request()
        .header("content-type", "text/xml")
        .body("foo=bar");
    let res = req.reply(&form).await;
    assert_eq!(
        res.status(),
        415,
        "bad content-type should be 415 Unsupported Media Type"
    );
}

#[tokio::test]
async fn form_allows_charset() {
    let _ = pretty_env_logger::try_init();

    let form = warp::body::form::<Vec<(String, String)>>();

    let req = warp::test::request()
        .header(
            "content-type",
            "application/x-www-form-urlencoded; charset=utf-8",
        )
        .body("foo=bar");

    let vec = req.filter(&form).await.unwrap();
    let expected = vec![("foo".to_owned(), "bar".to_owned())];
    assert_eq!(vec, expected);
}

#[tokio::test]
async fn form_invalid() {
    let _ = pretty_env_logger::try_init();

    let form = warp::body::form::<Vec<i32>>().map(|vec| warp::reply::json(&vec));

    let res = warp::test::request().body("nope").reply(&form).await;
    assert_eq!(res.status(), 400);
    let prefix = b"Request body deserialize error: ";
    assert_eq!(&res.body()[..prefix.len()], prefix);
}

#[tokio::test]
async fn form_enforce_strict_content_type() {
    let _ = pretty_env_logger::try_init();

    let form = warp::body::form_enforce_strict_content_type::<Vec<(String, String)>>()
        .map(|_| warp::reply());

    let req = warp::test::request().body("foo=bar&baz=quux");

    let res = req.reply(&form).await;
    assert_eq!(
        res.status(),
        415,
        "bad content-type should be 415 Unsupported Media Type"
    );

    let form = warp::body::form_enforce_strict_content_type::<Vec<(String, String)>>();

    let req = warp::test::request()
        .body("foo=bar&baz=quux")
        .header("content-type", "application/x-www-form-urlencoded");

    let vec = req.filter(&form).await.unwrap();
    let expected = vec![
        ("foo".to_owned(), "bar".to_owned()),
        ("baz".to_owned(), "quux".to_owned()),
    ];
    assert_eq!(vec, expected);
}

#[tokio::test]
async fn form_enforce_strict_content_type_rejects_bad_content_type() {
    let _ = pretty_env_logger::try_init();

    let form = warp::body::form_enforce_strict_content_type::<Vec<(String, String)>>()
        .map(|_| warp::reply());

    let req = warp::test::request()
        .header("content-type", "application/x-www-form-urlencoded")
        .body("foo=bar");

    let res = req.reply(&form).await;
    assert_eq!(res.status(), 200);

    let req = warp::test::request()
        .header("content-type", "text/xml")
        .body("foo=bar");
    let res = req.reply(&form).await;
    assert_eq!(
        res.status(),
        415,
        "bad content-type should be 415 Unsupported Media Type"
    );
}

#[tokio::test]
async fn form_enforce_strict_content_type_allows_charset() {
    let _ = pretty_env_logger::try_init();

    let form = warp::body::form_enforce_strict_content_type::<Vec<(String, String)>>();

    let req = warp::test::request()
        .header(
            "content-type",
            "application/x-www-form-urlencoded; charset=utf-8",
        )
        .body("foo=bar");

    let vec = req.filter(&form).await.unwrap();
    let expected = vec![("foo".to_owned(), "bar".to_owned())];
    assert_eq!(vec, expected);
}

#[tokio::test]
async fn form_enforce_strict_content_type_invalid() {
    let _ = pretty_env_logger::try_init();

    let form = warp::body::form_enforce_strict_content_type::<Vec<i32>>()
        .map(|vec| warp::reply::json(&vec));

    let res = warp::test::request()
        .body("nope")
        .header("content-type", "application/x-www-form-urlencoded")
        .reply(&form)
        .await;
    assert_eq!(res.status(), 400);
    let prefix = b"Request body deserialize error: ";
    assert_eq!(&res.body()[..prefix.len()], prefix);
}

#[tokio::test]
async fn stream() {
    let _ = pretty_env_logger::try_init();

    let stream = warp::body::stream();

    let body = warp::test::request()
        .body("foo=bar")
        .filter(&stream)
        .await
        .expect("filter() stream");

    let bufs: Result<Vec<_>, warp::Error> = body.try_collect().await;
    let bufs = bufs.unwrap();

    assert_eq!(bufs.len(), 1);
    assert_eq!(bufs[0].bytes(), b"foo=bar");
}
