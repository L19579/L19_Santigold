use {
    L19_Santigold::{
        run, get_configuration,
        Connection, PgPool,
    },
}; 

#[tokio::main]
async fn main() -> std::io::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default()
        .default_filter_or("trace")).init();
    
    let config = get_configuration()
        .expect("Failed to read config file");
    
    let db_conn_pool = PgPool::connect(&config.database_connection_string())
        .await
        .expect("Failed to connect to Postgres");

    let address = format!("127.0.0.1:{}", config.application_port);
    log::info!("Starting server! Listening at: {}", address);
    let listener = std::net::TcpListener::bind(address)?; 
    return run(listener, db_conn_pool)?.await;
}
