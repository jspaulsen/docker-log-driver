use std::{
    collections::HashMap, 
    sync::Arc,
};
use axum::{
    routing::post,
    Router,
};
use tokio::sync::{
    Mutex,
    oneshot::Sender,
};

use crate::{task::FifoProcessor, config::Config};


mod log_driver;
mod plugin;


#[derive(Clone)]
pub struct AppState {
    // maintain a shared mapping of log file paths to signal flags
    flags: Arc<Mutex<HashMap<String, Sender<bool>>>>,
    config: Config,
}

impl AppState {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            flags: Arc::new(
                Mutex::new(
                    HashMap::new()
                )
            ),
        }
    }

    pub async fn add_task_flag<S: Into<String>>(&mut self, fpath: S, flag: Sender<bool>) -> Option<Sender<bool>> {
        self
            .flags
            .lock()
            .await
            .insert(fpath.into(), flag)
    }

    pub async fn take_task_flag(&mut self, fpath: &str) -> Option<Sender<bool>> {
        self
            .flags
            .lock()
            .await
            .remove(fpath)
    }
}


pub struct Api<T> {
    _marker: std::marker::PhantomData<T>,
    state: AppState,
}


impl<T: FifoProcessor + Send + 'static> Api<T> {
    pub fn new(config: Config) -> Self {
        Self {
            _marker: std::marker::PhantomData,
            state: AppState::new(config),
        }
    }

    #[cfg(test)]
    pub fn from_existing_state(state: AppState) -> Self {
        Self {
            _marker: std::marker::PhantomData,
            state: state,
        }
    }

    pub fn into_router(self) -> Router {
        Router::new()
            .route("/Plugin.Activate", post(plugin::Plugin::activate))
            .route("/LogDriver.StartLogging", post(log_driver::LogDriver::start_logging::<T>))
            .route("/LogDriver.StopLogging", post(log_driver::LogDriver::stop_logging))
            .with_state(self.state)
    }
}


impl<T: FifoProcessor + Send + 'static> Into<Router> for Api<T> {
    fn into(self) -> Router {
        self.into_router()
    }
}
