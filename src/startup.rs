//! src/startup.rs

use actix_web::dev::Server;
use actix_web::{web, App, HttpServer};
use sqlx::PgPool;
use std::net::TcpListener;
use actix_web::middleware::Logger;

use crate::routes;

// we need to mark run as public.
// is no longer a binary entrypoint, therefore we can mark it as async without having to use any
// proc-macro incantation.
pub fn run(listener: TcpListener, connection_pool: PgPool) -> Result<Server, std::io::Error> {
    let connection = web::Data::new(connection_pool);

    let server = HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            .route("/health_check", web::get().to(routes::health_check))
            .route("/subscriptions", web::post().to(routes::subscribe))
            // get a pointer copy and attach it to the app state
            .app_data(connection.clone())
    })
    .listen(listener)?
    .run();

    Ok(server)
}
