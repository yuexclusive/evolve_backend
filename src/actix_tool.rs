// #![allow(unused)]

// use actix_files::{Files, NamedFile};
// use actix_web::web::Query;
// use actix_web::{get, Result};
// use openssl::ssl::{SslAcceptor, SslAcceptorBuilder, SslFiletype, SslMethod};
// use serde::Deserialize;


// fn ssl_builder() -> SslAcceptorBuilder {
//     // `openssl req -x509 -newkey rsa:4096 -nodes -keyout key.pem -out cert.pem -days 365 -subj '/CN=localhost'`
//     let mut builder = SslAcceptor::mozilla_intermediate(SslMethod::tls()).unwrap();
//     builder
//         .set_private_key_file("key.pem", SslFiletype::PEM)
//         .unwrap();
//     builder.set_certificate_chain_file("cert.pem").unwrap();
//     builder
// }

// #[derive(Deserialize)]
// pub struct FilePath {
//     pub path: String,
// }

// #[get("/open_file")]
// pub async fn open_file(file_path: Query<FilePath>) -> Result<NamedFile> {
//     let res = NamedFile::open(&file_path.path)?;
//     Ok(res)
// }

// pub async fn dir() -> Files {
//     actix_files::Files::new("/file", ".").show_files_listing()
// }

