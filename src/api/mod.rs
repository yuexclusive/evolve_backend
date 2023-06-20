pub mod user;

#[macro_export]
macro_rules! serve_api {
    ($app: expr) => {
        use crate::api::user;
        $app = $app.service(
            scope("/api")
                .service(ping)
                .service(user::login)
                .service(user::register)
                .service(user::send_email_code)
                .service(user::validate_exist_email)
                .service(user::validate_not_exist_email)
                .service(user::change_pwd)
                .service(
                    scope("/user")
                        .wrap(middleware::auth::Auth)
                        .service(user::search)
                        .service(user::update)
                        .service(user::get_current_user)
                        .service(user::delete)
                        .service(user::get),
                ),
        );
    };
}
