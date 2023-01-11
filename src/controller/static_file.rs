use actix_files::{Files, NamedFile};
use actix_web::{get, web::Path, Responder, Result};

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

// pub fn src_files() -> Files {
//     Files::new("/src", "./src")
//         .show_files_listing()
//         .use_last_modified(true)
// }
