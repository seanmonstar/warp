use serde::Deserialize;
use std::collections::HashMap;
use warp::Filter;

#[derive(Debug, Deserialize)]
struct TestForm {
    name: Option<String>,
    age: Option<u16>,
}

#[tokio::main]
async fn main() {
    // POST /form1 "name=haha"
    let form1 =
        warp::path!("form1")
            .and(warp::body::form())
            .map(|form: HashMap<String, String>| {
                let s = format!("name {:?}", form.get("name"));
                s
            });

    // POST /form2 "name=haha&age=10"
    let form2 = warp::path!("form2")
        .and(warp::body::form())
        .map(|form: TestForm| {
            let s = format!("name: {:?} age: {:?}", form.name, form.age);
            s
        });

    let routes = form1.or(form2);
    warp::serve(routes).run(([127, 0, 0, 1], 3030)).await;
}
