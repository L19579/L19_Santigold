mod routes;
mod configuration;

pub use {
    log,
    tracing,
    uuid::Uuid,
    actix_cors::Cors,
    actix_multipart::Multipart,
    aws_credential_types::Credentials,
    sqlx::{
        Connection, PgPool,
    },
    actix_web::{
        web, App,
        HttpRequest, HttpServer,
        Responder, HttpResponse,
        middleware, dev::Server,
        http::header::ContentType,
    },
    aws_sdk_s3::{
        model::ObjectCannedAcl,
        presigning::config::PresigningConfig,
        Client as S3Client, Config, Region, 
        types::{
            ByteStream, AggregatedBytes, 
        },
    },
    aws_smithy_http::{
        body::SdkBody
    },
    std::{
        net::TcpListener,
        sync::{
            Arc, RwLock,
        },
    },
    routes::{
        podcast::*,
        health_check::*,
    },
    configuration::*,
};

pub fn run(listener: TcpListener, db_conn_pool: PgPool, s3_client: S3)
-> Result::<Server, std::io::Error>{
    let xmls = Arc::new(RwLock::new(Xml::initialize()));
    let xmls = web::Data::new(xmls);
    let db_conn_pool = web::Data::new(db_conn_pool);
    let s3_client = web::Data::new(s3_client);
    let json_config = web::JsonConfig::default()
        .limit(50096) // raise this max TODO.
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
            .route("/health_check_xml", web::get().to(health_check_xml))
            .route("/health_check_xml_extended", web::get().to(health_check_xml_extended))
            .route("/health_check_xml_extended_post", web::post().to(health_check_xml_extended_post))
            .route("/feed", web::get().to(feed))
            .route("/upload_object", web::post().to(upload_object))
            .route("/upload_form", web::post().to(upload_form))
            .app_data(json_config.clone())
            .app_data(db_conn_pool.clone())
            .app_data(s3_client.clone())
            .app_data(xmls.clone())
    })
    .listen(listener)?
    .run();
    return Ok(server);
}
