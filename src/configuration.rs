use config::{
    Config, ConfigError,
    File, FileFormat,
};

pub fn get_configuration() -> Result<Settings, ConfigError>{
    let config = Config::builder()
        .add_source(File::new("configuration"), FileFormat::toml)
        .build()?
        .try_deserialize();
}

#[derive(serde::Deserialize)]
pub struct  Settings{
    pub application_port: String,
}

#[derive(serde::Deserialize)]
pub struct Database{
    pub host: String,
    pub port: String,
    pub username: String, 
    pub password: String,
    pub database_name: String,
    pub linode_object_storage: String,
}
