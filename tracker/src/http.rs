use crate::{api, liquidity_service::LiquidityService, pg::Database};
use actix_web::{App, HttpServer, web};

pub fn create_http_server(server_addr: &str, db: &Database) -> actix_web::dev::Server {
    let liquidity_service = LiquidityService::new(db.clone());
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(liquidity_service.clone()))
            .configure(api::configure)
    })
    .workers(2)
    .bind(server_addr)
    .expect("failed to bind API address")
    .run()
}
