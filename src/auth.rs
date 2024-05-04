use lazy_static::lazy_static;
use std::env;

use dotenvy::dotenv;
use regex::Regex;
use rocket::{
    http::{Cookie, Status},
    request::{FromRequest, Outcome},
};

lazy_static! {
    static ref AUTH_RE: Regex =
        Regex::new(r"(?i)basic\s*(?-i)(?P<tok>[^;]+)(;(?P<user_id>\d+))?").unwrap();
}

pub struct AuthUser(pub i64);

#[derive(Debug)]
pub struct AuthUserError;

#[rocket::async_trait]
impl<'r> FromRequest<'r> for AuthUser {
    type Error = AuthUserError;

    async fn from_request(req: &'r rocket::Request<'_>) -> Outcome<Self, Self::Error> {
        dotenv().ok();
        let secret = env::var("SECRET").expect("SECRET must be set");
        let key;
        let id;
        key = req.query_value::<String>("api-key");
        id = req.query_value::<i64>("user-id");
        let mut found_id = None;

        if let Some(Ok(key)) = key {
            if let Some(Ok(id)) = id {
                if key == secret {
                    found_id = Some(id);
                }
            }
        }

        if let Some(cookie) = req.cookies().get("secret") {
            let mut segs = cookie.value().split(';');
            if let Some(scrt) = segs.next() {
                if let Some(id) = segs.next() {
                    if let Ok(id) = id.parse() {
                        if scrt == secret {
                            found_id = Some(id);
                        }
                    }
                }
            }
        }

        if let Some(id) = found_id {
            let cookie = Cookie::build("secret", secret)
                .path("/")
                .secure(true)
                .http_only(true)
                .finish();

            req.cookies().add(cookie);
            return Outcome::Success(AuthUser(id));
        }

        Outcome::Failure((Status::Unauthorized, AuthUserError))
    }
}
