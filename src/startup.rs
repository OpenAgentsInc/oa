use crate::routes::{health_check, share_chat, subscribe};
use actix_files as fs;
use actix_web::dev::Server;
use actix_web::{web, App, HttpServer};
use sqlx::PgPool;
use std::net::TcpListener;

pub fn run(listener: TcpListener, db_pool: PgPool) -> Result<Server, std::io::Error> {
    let db_pool = web::Data::new(db_pool);
    let server = HttpServer::new(move || {
        App::new()
            .route("/health_check", web::get().to(health_check))
            .route("/api/subscriptions", web::post().to(subscribe))
            .route("/api/v1/chats/{chat_id}/share", web::post().to(share_chat))
            .app_data(db_pool.clone())
            // Serve principles.html at /principles
            .service(fs::Files::new("/principles", "./public").index_file("principles.html"))
            // Serve static files from the public directory
            .service(fs::Files::new("/", "./public").index_file("index.html"))
    })
    .listen(listener)?
    .run();
    Ok(server)
}
