use envconfig::Envconfig;
use tracing::Level;


#[derive(Envconfig, Debug, Clone)]
pub struct Config {
    #[envconfig(from = "LOG_INGEST_API", default = "http://localhost:8080")]
    pub log_ingest_api: String,

    #[envconfig(from = "LOG_LEVEL", default = "info")]
    pub log_level: Level,
}
