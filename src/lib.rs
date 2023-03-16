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
    actix_multipart::{
        form::{
            MultipartForm,
            MultipartCollect,
            MultipartFormConfig,
            json::Json as MultipartFormJson,
            text::Text as MultipartFormText,
            tempfile::TempFile as MultipartFormTempFile,
        },
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
        auth::*,
        podcast::*,
        health_check::*,
    },
    configuration::*,
};

//TODO inital attempt @ avoiding config in persistent data pool was wrong.
//Refactor. // Also don't need Arc<>; web::Data does the job.
pub fn run(listener: TcpListener, db_conn_pool: PgPool, s3_client: S3, admin_pass: AdminPassword)
-> Result::<Server, std::io::Error>{
    let admin_pass = web::Data::new(admin_pass.clone());
    let active_tokens = web::Data::new(RwLock::new(ActiveTokens(Vec::new())));
    log::info!("TRACE --------------------------------------- run 0");
    let xmls = Arc::new(RwLock::new(Xml::initialize(db_conn_pool.clone())));
    log::info!("TRACE --------------------------------------- run 1");
    let xmls = web::Data::new(xmls);
    log::info!("TRACE --------------------------------------- run 2");
    let db_conn_pool = web::Data::new(db_conn_pool);
    let s3_client = web::Data::new(s3_client);
    log::info!("TRACE --------------------------------------- run 3");
    let json_config = web::JsonConfig::default()
        .limit(50096) // raise this max TODO.
        .content_type(|mime| mime == mime::APPLICATION_JSON)
        .error_handler(|err, _req|{
            println!("Calling run--json_config--error");
            actix_web::error::InternalError::from_response(
                err, HttpResponse::Conflict().into()
            ).into()
        });
    let multipart_form_config = MultipartFormConfig::default()
        .error_handler(|err, req|{
            log::info!("TRACE Multipart ----- Bad request. Headers: {:?}",
                       req.headers());
            actix_web::error::InternalError::from_response(
                err, HttpResponse::BadRequest().into()
            ).into()

        });
    log::info!("TRACE --------------------------------------- run 4");
    let server = HttpServer::new(move ||{
        App::new()
            .wrap(middleware::Logger::default())
            .wrap(Cors::permissive()) // TODO CRTITICAL: temp
            .route("/health_check", web::get().to(health_check))
            .route("/health_check_xml", web::get().to(health_check_xml))
            .route("/health_check_xml_extended", web::get().to(health_check_xml_extended))
            .route("/health_check_xml_extended_post", web::post().to(health_check_xml_extended_post))
            .route("/channels", web::get().to(channels))
            .route("/podcast/{ch_title}", web::get().to(podcast))
            .route("/upload_object", web::post().to(upload_object))
            .route("/upload_form", web::post().to(upload_form))
            .route("/upload", web::post().to(upload))
            .route("/get_auth", web::post().to(generate_session_token))
            .app_data(json_config.clone())
            .app_data(multipart_form_config.clone())
            .app_data(db_conn_pool.clone())
            .app_data(s3_client.clone())
            .app_data(xmls.clone())
            .app_data(admin_pass.clone())
            .app_data(active_tokens.clone())
    })
    .listen(listener)?
    .run();
    log::info!("TRACE --------------------------------------- run END");
    return Ok(server);
}
