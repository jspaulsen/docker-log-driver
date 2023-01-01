use axum::Router;
use envconfig::Envconfig;

use client::IngestClient;
use config::Config;
use task::Task;

mod api;
mod client;
mod config;
mod log;
mod reader;
mod server;
mod task;


#[tokio::main]
async fn main() {
    let config = Config::init_from_env()
        .expect("Failed to initialize config");
    
    let server = server::UnixServer::new("ingest.sock") // TODO: changes to from_fpath
        .into_server()
        .unwrap();
    

    let app: Router = api::Api::<Task<IngestClient>>::new(config)
        .into_router();

    server
        .serve(app.into_make_service())
        .await
        .unwrap();
}
