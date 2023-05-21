use dotenvy::dotenv;
use rocket::{
    http::{Cookie, Status},
    request::{FromRequest, Outcome},
};
use std::env;

pub struct AuthUser(pub i64);

#[derive(Debug)]
pub struct AuthUserError;

#[rocket::async_trait]
impl<'r> FromRequest<'r> for AuthUser {
    type Error = AuthUserError;

    async fn from_request(req: &'r rocket::Request<'_>) -> Outcome<Self, Self::Error> {
        // Load environment data from .env file in root directory
        dotenv().ok();
        let secret = env::var("SECRET").expect("SECRET must be set");

        // Try to retrieve the user id and access key from the request parameters
        let mut found_id = None;
        if let Some(Ok(key)) = req.query_value::<String>("api_key") {
            if let Some(Ok(id)) = req.query_value::<i64>("user_id") {
                if key == secret {
                    found_id = Some(id);
                }
            }
        }

        // Try to retrieve the user id and access key from the request cookies
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

        // If the id is valid from either method, build a response cookie
        // and authorize the request
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
