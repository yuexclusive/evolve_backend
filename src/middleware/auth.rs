use std::future::{ready, Ready};

use crate::service::user as user_service;

use actix_web::{
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    http::header::{HeaderName, HeaderValue},
    Error,
};
use futures_util::future::LocalBoxFuture;
use util_error::unauthorized;

// There are two steps in middleware processing.
// 1. Middleware initialization, middleware factory gets called with
//    next service in chain as parameter.
// 2. Middleware's call method gets called with normal request.
pub struct Auth;

// Middleware factory is `Transform` trait
// `S` - type of the next service
// `B` - type of response's body
impl<S, B> Transform<S, ServiceRequest> for Auth
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = AuthMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(AuthMiddleware { service }))
    }
}

pub struct AuthMiddleware<S> {
    service: S,
}

impl<S, B> Service<ServiceRequest> for AuthMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, mut req: ServiceRequest) -> Self::Future {
        let authentication = req.headers().get("token");
        match authentication {
            Some(v) => match v.to_str() {
                Ok(v) => {
                    let token = v.to_string();
                    match user_service::check(&token) {
                        Ok(email) => {
                            req.headers_mut().insert(
                                HeaderName::from_static("email"),
                                HeaderValue::from_str(&email).unwrap(),
                            );

                            let fut = self.service.call(req);

                            Box::pin(async move {
                                let res = fut.await?;
                                Ok(res)
                            })
                        }
                        Err(err) => Box::pin(async move { Err(err.into()) }),
                    }
                }
                Err(err) => Box::pin(async move { Err(unauthorized!(err).into()) }),
            },
            None => Box::pin(async move { Err(unauthorized!("unauthorized").into()) }),
        }
    }
}
