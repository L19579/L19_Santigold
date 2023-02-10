mod routes;
mod configuration;

pub use {
    log,
    tracing,
    uuid::Uuid,
    actix_cors::Cors,
    sqlx::{
        Connection, PgPool,
    },
    actix_web::{
        web, App,
        HttpRequest, HttpServer,
        Responder, HttpResponse,
        middleware, dev::Server,
    },
    std::{
        net::TcpListener,
    },
    routes::{
        podcast::*,
        health_check::*,
    },
    configuration::*,
};

pub fn run(listener: TcpListener, db_conn_pool: PgPool)
-> Result::<Server, std::io::Error>{
    let db_conn_pool = web::Data::new(db_conn_pool);
    let json_config = web::JsonConfig::default()
        .limit(10096) // raise this max TODO.
        .content_type(|mime| mime == mime::APPLICATION_JSON)
        .error_handler(|err, _req|{
            println!("Calling run--json_config--error");
            actix_web::error::InternalError::from_response(
                err, HttpResponse::Conflict().into()
            ).into()
        });
    let server = HttpServer::new(move ||{
        // TODO: add logger/tracing
        App::new()
            .wrap(middleware::Logger::default())
            .wrap(Cors::permissive()) // TODO CRTITICAL: temp
            .route("/health_check", web::get().to(health_check))
            .route("/feed", web::get().to(feed))
            .route("/post_episode", web::post().to(post_episode))
            .app_data(json_config.clone())
            .app_data(db_conn_pool.clone())
    })
    .listen(listener)?
    .run();
    return Ok(server);
}
