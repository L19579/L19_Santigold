use config::{
    Config, ConfigError,
    File, FileFormat,
};

pub fn get_configuration() -> Result<Settings, ConfigError>{
    let config = Config::builder()
        .add_source(File::new("real_configuration", FileFormat::Toml))
        .build()?
        .try_deserialize::<Settings>()?;

    return Ok(config); 
}

#[derive(serde::Deserialize, Clone)]
pub struct Settings{
    pub in_production_mode: bool,
    pub application_port: String,
    pub temp_dir: String,
    pub database: DatabaseSettings,
    pub s3_bucket: S3Bucket,
}

impl Settings{
    pub fn database_connection_string(&self) -> String{
        let database_name = if self.in_production_mode{
            &self.database.production_database_name
        } else {
            &self.database.test_database_name
        };
        return format!("postgres://{}:{}@{}:{}/{}",
                       self.database.username, self.database.password, 
                       self.database.host, self.database.port, database_name);
    } 
}

#[derive(serde::Deserialize, Clone)]
pub struct DatabaseSettings{
    pub host: String,
    pub port: String,
    pub username: String, 
    pub password: String,
    pub production_database_name: String,
    pub test_database_name: String,
}

#[derive(serde::Deserialize, Clone)]
pub struct S3Bucket{
    pub region: String,
    pub bucket: String,
    pub endpoint_url: String,
    pub access_key: String,
    pub secret_access_key: String,
}

impl S3Bucket{
    pub fn full_link(&self) -> String{
        let mut link = self.endpoint_url.clone().to_lowercase();
        link = link.replace("https://", &format!("https://{}.", self.bucket)); 
        return link;
    }
}
