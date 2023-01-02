use async_trait::async_trait;
use reqwest;
use serde_json::Value;

use crate::log::LogMessage;



#[async_trait]
pub trait Ingest {
    fn new<S: AsRef<str>>(uri: S) -> Self where Self: Sized;
    async fn ingest(&mut self, message: LogMessage) -> Result<serde_json::Value, reqwest::Error>;
}

pub struct IngestClient {
    uri: String,
}

#[async_trait]
impl Ingest for IngestClient {
    fn new<S: AsRef<str>>(uri: S) -> Self where Self: Sized {
        Self {
            uri: uri
                .as_ref()
                .to_string(),
        }
    }

    async fn ingest(&mut self, message: LogMessage) -> Result<serde_json::Value, reqwest::Error> {
        let client = reqwest::Client::new();
        let url = format!("{}/logs", self.uri);

        let response = client
            .post(url)
            .json(&vec![&message])
            .send()
            .await?;
        
        response
            .json::<Value>()
            .await
    }
}
