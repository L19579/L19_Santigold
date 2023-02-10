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

#[derive(serde::Deserialize)]
pub struct  Settings{
    pub production_mode: bool,
    pub application_port: String,
    pub database: DatabaseSettings,
}

impl Settings{
    pub fn connection_string(&self) -> String{
        let database_name = if self.production_mode{
            self.database.production_database_name
        } else {
            self.database.test_database_name
        };
        return format!("postgres://{}:{}@{}:{}/{}",
                       self.database.username, self.database.password, 
                       self.database.host, self.database.port, database_name);
    } 
}

#[derive(serde::Deserialize)]
pub struct DatabaseSettings{
    pub host: String,
    pub port: String,
    pub username: String, 
    pub password: String,
    pub production_database_name: String,
    pub test_database_name: String,
    pub linode_object_storage: String,
}
