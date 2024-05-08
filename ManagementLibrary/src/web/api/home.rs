use actix_web::{Scope, web};

pub fn initialize() -> Scope {
    web::scope("/")
}
