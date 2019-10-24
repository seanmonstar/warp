#![deny(warnings)]
use warp::Filter;

#[tokio::test]
async fn flattens_tuples() {
    let _ = pretty_env_logger::try_init();

    let str1 = warp::any().map(|| "warp");
    let true1 = warp::any().map(|| true);
    let unit1 = warp::any();

    // just 1 value
    let ext = warp::test::request().filter(&str1).await.unwrap();
    assert_eq!(ext, "warp");

    // just 1 unit
    let ext = warp::test::request().filter(&unit1).await.unwrap();
    assert_eq!(ext, ());

    // combine 2 values
    let and = str1.and(true1);
    let ext = warp::test::request().filter(&and).await.unwrap();
    assert_eq!(ext, ("warp", true));

    // combine 2 reversed
    let and = true1.and(str1);
    let ext = warp::test::request().filter(&and).await.unwrap();
    assert_eq!(ext, (true, "warp"));

    // combine 1 with unit
    let and = str1.and(unit1);
    let ext = warp::test::request().filter(&and).await.unwrap();
    assert_eq!(ext, "warp");

    let and = unit1.and(str1);
    let ext = warp::test::request().filter(&and).await.unwrap();
    assert_eq!(ext, "warp");

    // combine 3 values
    let and = str1.and(str1).and(true1);
    let ext = warp::test::request()
        .filter(&and)
        .await
        .unwrap();
    assert_eq!(ext, ("warp", "warp", true));

    // combine 2 with unit
    let and = str1.and(unit1).and(true1);
    let ext = warp::test::request()
        .filter(&and)
        .await
        .unwrap();
    assert_eq!(ext, ("warp", true));

    let and = unit1.and(str1).and(true1);
    let ext = warp::test::request()
        .filter(&and)
        .await
        .unwrap();
    assert_eq!(ext, ("warp", true));

    let and = str1.and(true1).and(unit1);
    let ext = warp::test::request()
        .filter(&and)
        .await
        .unwrap();
    assert_eq!(ext, ("warp", true));

    // nested tuples
    let str_true_unit = str1.and(true1).and(unit1);
    let unit_str_true = unit1.and(str1).and(true1);

    let and = str_true_unit.and(unit_str_true);
    let ext = warp::test::request()
        .filter(&and)
        .await
        .unwrap();
    assert_eq!(ext, ("warp", true, "warp", true));

    let and = unit_str_true.and(unit1).and(str1).and(str_true_unit);
    let ext = warp::test::request()
        .filter(&and)
        .await
        .unwrap();
    assert_eq!(ext, ("warp", true, "warp", "warp", true));
}

#[tokio::test]
async fn map() {
    let _ = pretty_env_logger::try_init();

    let ok = warp::any().map(warp::reply);

    let req = warp::test::request();
    let resp = req.reply(&ok).await;
    assert_eq!(resp.status(), 200);
}

#[tokio::test]
async fn unify() {
    let _ = pretty_env_logger::try_init();

    let a = warp::any().map(|| 1);
    let b = warp::any().map(|| 2);
    let or = a.or(b).unify();

    let ex = warp::test::request().filter(&or).await.unwrap();
    assert_eq!(ex, 1);
}
