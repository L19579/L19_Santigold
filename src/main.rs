use {
    L19_Santigold::{
        run, get_configuration,
        PgPool, S3, S3Client, 
        Config, Credentials, 
        Region, AdminPassword,
    },
}; 

#[tokio::main]
async fn main() -> std::io::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default()
        .default_filter_or("trace")).init();
    
    let config = get_configuration()
        .expect("Failed to read config file");

    let admin_pass = AdminPassword(config.admin_password.clone());

    let db_conn_pool = PgPool::connect(&config.database_connection_string())
        .await
        .expect("Failed to connect to Postgres");
  
    let s3_config = config.s3_bucket;
    let s3_credentials = Credentials::from_keys(
        &s3_config.access_key, &s3_config.secret_access_key, None);
    let s3_conf = Config::builder()
       .credentials_provider(s3_credentials)
       .endpoint_url(&s3_config.endpoint_url)
       .region(Region::new(s3_config.region.to_string()))
       .build();
    let s3 = S3{
        client: S3Client::from_conf(s3_conf),
        bucket: s3_config.bucket.to_string(),
        full_link: s3_config.full_link(),
        temp_dir: config.temp_dir,
    };

    let address = format!("0.0.0.0:{}", config.application_port);
    log::info!("Starting server! Listening at: {}", address);
    let listener = std::net::TcpListener::bind(address)?; 
    return run(listener, db_conn_pool, s3, admin_pass)?.await;
}
