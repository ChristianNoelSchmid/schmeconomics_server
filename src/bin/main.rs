#[macro_use]
extern crate rocket;
use schmeconomics_server::{controller, cors::Cors};

#[launch]
fn rocket() -> _ {
    let rckt = rocket::build().attach(Cors);
    let rckt = controller::routes(rckt);

    rckt
}
