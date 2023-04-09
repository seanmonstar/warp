use futures_util::TryStreamExt;
use warp::multipart::FormData;
use warp::Buf;
use warp::Filter;

#[tokio::main]
async fn main() {
    // Running curl -F file=@.gitignore 'localhost:3030/' should print [("file", ".gitignore", "\n/target\n**/*.rs.bk\nCargo.lock\n.idea/\nwarp.iml\n")]
    let route = warp::multipart::form().and_then(|form: FormData| async move {
        let field_names: Vec<_> = form
            .and_then(|mut field| async move {
                let contents =
                    String::from_utf8_lossy(field.data().await.unwrap().unwrap().chunk())
                        .to_string();
                Ok((
                    field.name().to_string(),
                    field.filename().unwrap().to_string(),
                    contents,
                ))
            })
            .try_collect()
            .await
            .unwrap();

        Ok::<_, warp::Rejection>(format!("{:?}", field_names))
    });
    warp::serve(route).run(([127, 0, 0, 1], 3030)).await;
}
