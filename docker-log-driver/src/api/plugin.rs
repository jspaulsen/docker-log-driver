use axum::Json;
use serde::Serialize;


pub struct Plugin;


#[derive(Serialize)]
pub struct PluginImplements {
    #[serde(rename = "Implements")]
    pub implements: Vec<String>,
}

impl PluginImplements {
    pub fn new() -> Self {
        Self {
            implements: vec!["LogDriver".to_string()],
        }
    }
}


impl Plugin {
    pub async fn activate() -> Json<PluginImplements> {
        Json(PluginImplements::new())
    }
}
