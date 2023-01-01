use axum::Router;
use envconfig::Envconfig;

use config::Config;
use task::ApiTask;

mod api;
mod client;
mod config;
mod error;
mod log;
mod reader;
mod server;
mod task;


#[tokio::main]
async fn main() {
    let config = Config::init_from_env()
        .expect("Failed to load configuration!");

    tracing_subscriber::fmt()
        .json()
        .with_env_filter(
            config
                .log_level
                .to_string()
        )
        .with_current_span(false)
        .init();
    
    let server = server::UnixServer::from_filename("ingest.sock")
        .into_server()
        .unwrap();
    

    let app: Router = api::Api::<ApiTask>::new(config)
        .into_router();

    server
        .serve(app.into_make_service())
        .await
        .unwrap();
}
