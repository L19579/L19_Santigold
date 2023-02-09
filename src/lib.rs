mod routes;
mod configuration;

pub use {
    log,
    tracing,
    uuid::Uuid,
    actix_cors::Cors,
    sqlx::{
        Connection, PgPool,
    }
    actix_web::{
        web, App,
        HttpRequest, HttpServer,
        Responder, HttpResponse,
        middleware, dev::Server,
    }
    std::{
        net::TcpListener,
    },
    routes::{
        podcast::*,
        health_check::*,
    },
    configuration::*,
};

pub fn run(listen: TcpListener, db_conn_pool: PgPool)
-> Result::<Server, std::io::Error>{
    // PgPool Decl here
    // Server Setup here
}
