use axum::{
    extract::{State, RawBody},
    Json,
    response::IntoResponse,
};
use serde::Deserialize;
use serde_json::json;
use tracing::{warn, info};

use crate::{task::FifoProcessor, error::HttpError};

use super::AppState;


#[derive(Deserialize)]
pub struct StartLoggingInfo {
    #[serde(rename = "ContainerID")]
    pub container_id: String,
}

#[derive(Deserialize)]
pub struct StartLoggingPayload {
    #[serde(rename = "File")]
    pub file: String,

    #[serde(rename = "Info")]
    pub info: StartLoggingInfo,
}

#[derive(Deserialize)]
pub struct StopLoggingPayload {
    #[serde(rename = "File")]
    pub file: String,
}

pub struct LogDriver;


impl LogDriver {
    pub async fn start_logging<T: FifoProcessor + Send + 'static>(
        State(mut state): State<AppState>,
        RawBody(payload): RawBody,
    ) -> Result<impl IntoResponse, HttpError> {
        let (tx, rx) = tokio::sync::oneshot::channel::<bool>();
        let payload: StartLoggingPayload = serde_json::from_slice(
            &hyper::body::to_bytes(payload)
                .await
                .map_err(|_| HttpError::bad_request(None))?
        ).map_err(|_| HttpError::bad_request(None))?;

        let task: T = T::new(
            state
                .config
                .clone()
        );

        state
            .add_task_flag(
                &payload.file, 
                tx,
        ).await;

        // Spawn the task to process the fifo file
        tokio::spawn(task.process(payload.file, rx));
        Ok(Json(json!({"Err": ""})))
    }

    pub async fn stop_logging(
        State(mut state): State<AppState>,
        RawBody(payload): RawBody,
    ) -> Result<impl IntoResponse, HttpError> {
        let payload: StopLoggingPayload = serde_json::from_slice(
            &hyper::body::to_bytes(payload)
                .await
                .map_err(|_| HttpError::bad_request(None))?
        ).map_err(|_| HttpError::bad_request(None))?;

        let flag = state
            .take_task_flag(&payload.file)
            .await;
        
        match flag {
            Some(flag) => {
                info!(
                    fpath = payload.file.as_str(),
                    "Found existing task for container logging to {}; sending stop signal", 
                    payload.file,
                );

                if let Err(_) = flag.send(true) {
                    warn!(
                        fpath = payload.file.as_str(),
                        "Signal receiver dropped; task panic, deadlocked or complete for container logging to {}", 
                        payload.file,
                    );
                }

                Ok(Json(json!({"Err": ""})))
            },
            None => {
                warn!(
                    fpath = payload.file.as_str(),
                    "No task found for container logging to {}; ignoring stop signal", 
                    payload.file,
                );

                Ok(Json(json!({"Err": "No task found for container logging to file"})))
            },
        }
    }
}


#[cfg(test)]
mod tests {
    use std::{
        collections::HashMap,
        path::PathBuf,
        sync::{
            Arc,
            Mutex,
        },
    };

    use axum::http;
    use envconfig::Envconfig;
    use hyper::{Request, Body};
    use lazy_static::lazy_static;    
    use tokio::sync::oneshot::Receiver;
    use tower::ServiceExt;

    use crate::{
        api::{Api, AppState},
        task::FifoProcessor, config::Config,
    };


    struct TestProcessor;

    #[async_trait::async_trait]
    impl FifoProcessor for TestProcessor {
        fn new(_: Config) -> Self {
            Self
        }

        async fn process<P: Into<PathBuf> + Send>(self, path: P, recv: Receiver<bool>) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
            let p: String= path
                .into()
                .to_str()
                .unwrap()
                .to_string();
            
            recv.await
                .expect("Failed to receive stop signal");
            
            HASHMAP.lock()
                .unwrap()
                .insert(p, true);

            Ok(())
        }
    }


    // this is going to be a stupid way to run these tests
    lazy_static! {
        static ref HASHMAP: Arc<Mutex<HashMap<String, bool>>> = {
            Arc::new(
                Mutex::new(
                    HashMap::new()
                )
            )
        };
    }

    async fn post<S: Into<String>>(path: S, state: AppState, body: serde_json::Value) -> hyper::Response<http_body::combinators::UnsyncBoxBody<axum::body::Bytes, axum::Error>> {
        let api = Api::<TestProcessor>::from_existing_state(state)
            .into_router();

        let request = Request::builder()
            .uri(path.into())
            .method(http::Method::POST)
            .header("Content-Type", "application/json")
            .body(Body::from(serde_json::to_string(&body).unwrap()))
            .expect("Failed to build request");
        
        api
            .oneshot(request)
            .await
            .unwrap()
    }

    fn config() -> Config {
        let hashmap = {
            let mut m = HashMap::new();

            m.insert("LOG_INGEST_API".to_string(), "http://localhost:8080".to_string());
            m
        };

        Config::init_from_hashmap(&hashmap)
            .expect("Failed to init config")
    }


    #[tokio::test]
    async fn test_start_logging() {
        let fpath = "/tmp/test_fifo";

        // Clear the hashmap for our path
        HASHMAP
            .lock()
            .unwrap()
            .remove(fpath);
        
        // define state here so channels aren't dropped
        let mut state = crate::api::AppState::new(config());

        let body = serde_json::json!({
            "File": fpath,
            "Info": {
                "ContainerID": "test_container_id",
            }
        });

        let results = post(
            "/LogDriver.StartLogging",
            state.clone(),
            body,
        ).await;

        assert_eq!(results.status(), http::StatusCode::OK);
        
        // retrieve flag from state
        let flag = state
            .take_task_flag(fpath)
            .await
            .expect("Failed to retrieve task flag");
        
        flag.send(true)
            .unwrap();
        
        // give task time to complete
        // again, this is an awful way to do this
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        // read the hashmap to see if the task was spawned
        let value = HASHMAP
            .lock()
            .unwrap()
            .get(fpath)
            .unwrap()
            .clone();
        
        assert_eq!(value, true);
            
    }

    #[tokio::test]
    async fn test_stop_logging() {
        let fpath = "/tmp/stop_logging_fifle";

        // Clear the hashmap for our path
        HASHMAP
            .lock()
            .unwrap()
            .remove(fpath);

        let state = crate::api::AppState::new(config());
        let body = serde_json::json!({
            "File": fpath,
            "Info": {
                "ContainerID": "test_container_id",
            }
        });

        let results = post(
            "/LogDriver.StartLogging",
            state.clone(),
            body,
        ).await;

        assert_eq!(results.status(), http::StatusCode::OK);

        // Call stop logging
        let body = serde_json::json!({
            "File": fpath,
        });

        let results = post(
            "/LogDriver.StopLogging",
            state.clone(),
            body,
        ).await;

        assert_eq!(results.status(), http::StatusCode::OK);
        
        // give task time to complete
        // again, this is an awful way to do this
        tokio::time::sleep(tokio::time::Duration::from_millis(20)).await;

        // read the hashmap to see if the task was spawned
        let value = HASHMAP
            .lock()
            .unwrap()
            .get(fpath)
            .unwrap()
            .clone();
        
        assert_eq!(value, true);
    }
}
