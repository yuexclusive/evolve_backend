#![cfg(feature = "upload_file")]

use std::io::Write;

use actix_multipart::Multipart;
use actix_web::{get, post, web, HttpResponse, Responder, Result};
use futures_util::TryStreamExt as _;
use std::path::Path;
use uuid::Uuid;

#[macro_export]
macro_rules! serve_upload_file {
    ($app: expr) => {
        $app = $app.service(
            scope("/file")
                .service(upload_file::upload_page)
                .service(upload_file::upload),
        );
    };
}

#[get("/upload")]
async fn upload_page() -> HttpResponse {
    let html = r#"<html>
        <head><title>Upload Test</title></head>
        <body>
            <form target="/" method="post" enctype="multipart/form-data">
                <input type="file" multiple name="file"/>
                <button type="submit">Submit</button>
            </form>
        </body>
    </html>"#;

    HttpResponse::Ok().body(html)
}

#[post("/upload")]
async fn upload(mut payload: Multipart) -> Result<impl Responder> {
    // iterate over multipart stream
    while let Some(mut field) = payload.try_next().await? {
        // A multipart/form-data stream has to contain `content_disposition`
        let content_disposition = field.content_disposition();

        let filename = content_disposition
            .get_filename()
            .map_or_else(|| Uuid::new_v4().to_string(), sanitize_filename::sanitize);

        let dir = "./.tmp";

        if !Path::new(dir).exists() {
            web::block(move || std::fs::create_dir(dir)).await??;
        }

        let filepath = Path::new(dir).join(filename);

        // File::create is blocking operation, use threadpool
        let mut f = web::block(|| {
            std::fs::File::options()
                .write(true)
                .create(true)
                .open(filepath)
        })
        .await??;

        // Field in turn is stream of *Bytes* object
        while let Some(chunk) = field.try_next().await? {
            // filesystem operations are blocking, we have to use threadpool
            f = web::block(move || f.write_all(&chunk).map(|_| f)).await??;
        }
    }

    HttpResponse::Ok().await
}
