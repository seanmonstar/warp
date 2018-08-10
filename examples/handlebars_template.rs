#![deny(warnings)]
extern crate warp;
extern crate hyper;
extern crate handlebars;
#[macro_use]
extern crate serde_json;

use warp::Filter;
use handlebars::Handlebars;
use std::sync::Arc;

fn main() {
    let template = "<!DOCTYPE html>
                    <html>
                      <head>
                        <title>Warp Handlebars template example</title>
                      </head>
                      <body>
                        <h1>Hello {{user}}!</h1>
                      </body>
                    </html>";
    
    let mut hb = Handlebars::new();
    // register the template
    hb.register_template_string("template.html", template).unwrap();

    // Turn Handlebars instance into a Filter so we can combine it
    // easily with others...
    let hb = Arc::new(hb);
    let hb = warp::any().map(move || hb.clone());

    //GET /
    let route = warp::get2()
        .and(warp::index())
        .and(hb)
        .map(render_index);

    warp::serve(route).run(([127, 0, 0, 1], 3030));
}

//GET / handler
fn render_index(hb: Arc<Handlebars>) -> impl warp::Reply {
    hb.render("template.html", &json!({"user": "Warp"})).unwrap()
}
