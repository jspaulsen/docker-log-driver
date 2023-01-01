use std::{path::PathBuf, future::IntoFuture};

use tokio::sync::oneshot::Receiver;
use tracing::info;

use crate::{client::Ingest, config::Config};



#[async_trait::async_trait]
pub trait FifoProcessor {
    fn new(config: Config) -> Self;
    async fn process<P: Into<PathBuf> + Send>(self, path: P, receiver: Receiver<bool>) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>>; // TODO: select appropriate error type
}

pub struct Task<T> {
    config: Config,
    _marker: std::marker::PhantomData<T>,
}


#[async_trait::async_trait]
impl<T: Ingest + Sync + Send> FifoProcessor for Task<T> {
    fn new(config: Config) -> Self {
        Self {
            config,
            _marker: std::marker::PhantomData,
        }
    }

    async fn process<P: Into<PathBuf> + Send>(self, path: P, receiver: Receiver<bool>) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
        let path = path.into();
        let path_str = path
            .to_str()
            .unwrap()
            .to_string();

        tokio::select! {
            _ = receiver => {
                info!(
                    fpath = path_str.as_str(),
                    "Received stop signal for {}", path_str,
                );

                Ok(())
            },
            results = process_file(path) => {
                results
            }
        }
    }
}


async fn process_file(path: PathBuf) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    let fp = tokio::fs::OpenOptions::new()
        .read(true)
        .open(path)
        .await?;

    let mut reader = crate::reader::Reader::new(fp);

    loop {
        let log_entry = reader
            .next()
            .await?;
        
        match log_entry {
            Some(entry) => {
                todo!()
            },
            None => {
                break;
            }
        }
    }
    // while let Ok(line) = reader.next().await {
    //     info!(line = line.as_str(), "Read line");
    // }
    
    // read the file
    // send the data to the ingest client
    // wait for the signal to stop
    todo!()
}

// pub fn new_task() -> Task {
//     let (sender, receiver) = tokio::sync::oneshot::channel::<bool>();

//     Task {
//         data: TaskData {
//             receiver,
//         },
//     }
// }
