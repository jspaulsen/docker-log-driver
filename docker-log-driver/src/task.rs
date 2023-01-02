use std::path::PathBuf;

use tokio::{
    io::AsyncReadExt,
    sync::oneshot::Receiver,
};
use tracing::info;

use crate::{
    client::{
        Ingest,
        IngestClient,
    },
    config::Config,
    error::Loggable,
    log::LogMessage,
};


pub type ApiTask = Task<IngestClient>;

#[async_trait::async_trait]
pub trait FifoProcessor {
    fn new(config: Config) -> Self;
    async fn process<P: Into<PathBuf> + Send>(self, path: P, receiver: Receiver<bool>) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>>; // TODO: select appropriate error type
}

pub struct Task<T> {
    config: Config,
    _t: std::marker::PhantomData<T>,
}


#[async_trait::async_trait]
impl<T: Ingest + Sync + Send> FifoProcessor for Task<T> {
    fn new(config: Config) -> Self {
        Self {
            config,
            _t: std::marker::PhantomData,
        }
    }

    async fn process<P: Into<PathBuf> + Send>(self, path: P, receiver: Receiver<bool>) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
        let path = path.into();
        let fp = tokio::fs::OpenOptions::new()
            .read(true)
            .open(&path)
            .await?;
        let fpath = format!("{:?}", path);

        tokio::select! {
            _ = receiver => {
                info!(
                    fpath = fpath,
                    "Received stop signal for {}", fpath,
                );

                Ok(())
            },
            results = process_file::<tokio::fs::File, T>(&self.config, fp) => {
                results
                    .log_error(format!("Processing file {} resulted in error", fpath))
            }
        }
    }
}


async fn process_file<A: AsyncReadExt, T: Ingest>(config: &Config, file: A) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let mut reader = crate::reader::Reader::new(file);
    let mut client = T::new(&config.log_ingest_api);

    loop {
        let log_entry = reader
            .next()
            .await?;
        
        // TODO: This isn't super efficient.  We should probably use a MPSC channel to send the messages
        // on a separate green thread.  For a first pass, this is fine.
        match log_entry {
            Some(entry) => {
                let message = LogMessage::try_from(entry)?; // TODO: select appropriate error type
                let results = client
                    .ingest(message)
                    .await;
                
                match results {
                    Ok(_) => {},
                    Err(e) => {
                        tracing::error!(
                            error = ?e,
                            "Error ingesting log message",
                        );
                    }
                }
            },
            None => { // If empty, we received EOF
                break; 
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use docker_protobuf::LogEntry;
    use envconfig::Envconfig;
    use lazy_static::lazy_static;
    use prost::Message;

    use crate::{log::LogMessage, client::Ingest, config::Config};

    use super::process_file;

    struct TestIngestClient;

    #[async_trait::async_trait]
    impl Ingest for TestIngestClient {
        fn new<S: AsRef<str>>(_: S) -> Self {
            Self
        }

        async fn ingest(&mut self, message: LogMessage) -> Result<serde_json::Value, reqwest::Error> {
            HASHMAP
                .lock()
                .unwrap()
                .entry(message.message.to_owned())
                .or_insert_with(Vec::new)
                .push(message);

            Ok(serde_json::json!({"count": 1}))
        }
    }

    #[derive(Default)]
    struct ReadBuilder {
        data: Vec<u8>,
    }

    impl ReadBuilder {
        fn add(mut self, entry: LogEntry) -> Self {
            let mut buf = Vec::new();

            entry.encode(&mut buf)
                .unwrap();

            let entry_size = buf.len() as u32;
            let size_buf = entry_size.to_be_bytes();
            
            self.data.append(&mut size_buf.to_vec());
            self.data.append(&mut buf);

            self
        }

        fn build(self) -> Vec<u8> {
            self.data
        }
    }
    
    fn config() -> Config {
        let hashmap = {
            let mut m = HashMap::new();

            m.insert(
                "LOG_INGEST_API".to_string(),
                "http://localhost:8080".to_string(),
            );
            
            m
        };

        Config::init_from_hashmap(&hashmap)
            .unwrap()
    }


    // this is going to be a stupid way to run these tests
    lazy_static! {
        static ref HASHMAP: std::sync::Arc<std::sync::Mutex<HashMap<String, Vec<LogMessage>>>> = {
            std::sync::Arc::new(
                std::sync::Mutex::new(
                    HashMap::new()
                )
            )
        };
    }

    #[tokio::test]
    async fn test_process_file() {
        let test_key = "test_process_file".to_string();
        let data = ReadBuilder::default()
            .add(LogEntry {
                source: "test".to_string(),
                time_nano: 0,
                line: test_key
                    .as_bytes()
                    .to_vec(),
                partial: false,
                partial_log_metadata: None,
            }).add(LogEntry {
                source: "test".to_string(),
                time_nano: 0,
                line: test_key
                    .as_bytes()
                    .to_vec(),
                partial: false,
                partial_log_metadata: None,
            }).build();
        let config = config();

        // clear out existing hashmap key
        HASHMAP
            .lock()
            .unwrap()
            .remove(&test_key);

        process_file::<&[u8], TestIngestClient>(&config, &data[..])
            .await
            .expect("Processing file should not result in error");
        
        // read contents of hashmap via key
        let messages = HASHMAP
            .lock()
            .unwrap()
            .get(&test_key)
            .expect("Should have received messages")
            .clone();

        assert_eq!(messages.len(), 2);
    }
}
