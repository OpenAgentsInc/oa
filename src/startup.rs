use crate::routes::{health_check, subscribe};
use actix_files as fs;
use actix_web::dev::Server;
use actix_web::web::Data;
use actix_web::{web, App, HttpServer};
use sqlx::PgPool;
use std::net::TcpListener;
use tracing::info;
use tracing_actix_web::TracingLogger;

pub fn run(listener: TcpListener, db_pool: PgPool) -> Result<Server, std::io::Error> {
    let db_pool = Data::new(db_pool);
    let port = listener.local_addr().unwrap().port();

    // Log the server URL
    info!(
        "Server started! Access the web interface at http://localhost:{}",
        port
    );

    let server = HttpServer::new(move || {
        App::new()
            .wrap(TracingLogger::default())
            // Health check route without /api prefix
            .route("/health_check", web::get().to(health_check))
            // API routes under /api prefix
            .service(
                web::scope("/api")
                    .route("/subscriptions", web::post().to(subscribe))
                    .app_data(db_pool.clone()),
            )
            // Serve principles.html at /principles
            .service(fs::Files::new("/principles", "./public").index_file("principles.html"))
            // Serve static files from the public directory
            .service(fs::Files::new("/", "./public").index_file("index.html"))
    })
    .listen(listener)?
    .run();
    Ok(server)
}