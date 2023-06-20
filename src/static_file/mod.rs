#![cfg(feature = "static_file")]

use actix_files::{Files, NamedFile};
use actix_web::{get, web::Path, Responder, Result};

#[macro_export]
macro_rules! serve_static_file {
    ($app: expr) => {
        $app = $app.service(
            scope("/static")
                .service(scope("/single").service(static_file::file))
                .service(static_file::static_files()), // .service(controller::static_file::src_files()),
        );
    };
}

#[get("/{filename:.*}")]
async fn file(name: Path<String>) -> Result<impl Responder> {
    let res = NamedFile::open(name.into_inner())?;
    Ok(res)
}

pub fn static_files() -> Files {
    Files::new("/static", "./static")
        .show_files_listing()
        .use_last_modified(true)
}
