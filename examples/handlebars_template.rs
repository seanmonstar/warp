#![deny(warnings)]
use handlebars::Handlebars;
use serde_json::json;
use std::sync::Arc;
use warp::Filter;

#[tokio::main]
async fn main() {
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
    // Register the template
    hb.register_template_string("template.html", template)
        .unwrap();

    // Turn the Handlebars instance into a Filter
    // so we can combine it easily with others...
    let hb = Arc::new(hb);

    // Create a reusable closure to render template
    let render = move |name| {
        // A closure that takes a `Serialize`able value,
        // and passes it into the template engine
        move |value| hb.render(name, &value).unwrap_or_else(|e| e.to_string())
    };

    //GET /
    let route = warp::get()
        .and(warp::path::end())
        .map(|| json!({
                "user" : "Warp"
        }))
        .map(render("template.html"));

    warp::serve(route).run(([127, 0, 0, 1], 3030)).await;
}
