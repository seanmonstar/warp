#![deny(warnings)]
extern crate handlebars;
extern crate hyper;
extern crate warp;
#[macro_use]
extern crate serde_json;
extern crate serde;

use std::error::Error;
use std::sync::Arc;

use handlebars::Handlebars;
use serde::Serialize;
use warp::Filter;

struct WithTemplate<T: Serialize> {
    name: &'static str,
    value: T,
}

fn render<T>(template: WithTemplate<T>, hbs: Arc<Handlebars>) -> impl warp::Reply
where
    T: Serialize,
{
    hbs.render(template.name, &template.value)
        .unwrap_or_else(|err| err.description().to_owned())
}

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
    hb.register_template_string("template.html", template)
        .unwrap();

    // Turn Handlebars instance into a Filter so we can combine it
    // easily with others...
    let hb = Arc::new(hb);

    // Create a reusable closure to render template
    let handlebars = move |with_template| render(with_template, hb.clone());

    //GET /
    let route = warp::get2()
        .and(warp::path::end())
        .map(|| WithTemplate {
            name: "template.html",
            value: json!({"user" : "Warp"}),
        })
        .map(handlebars);

    warp::serve(route).run(([127, 0, 0, 1], 3030));
}
