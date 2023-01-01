use chrono::{
    NaiveDateTime, 
    DateTime, 
    Utc,
};
use docker_protobuf::LogEntry;
use serde::Serialize;
use serde_json::{
    Value,
    Number,
};



#[derive(Debug, Clone, Serialize)]
pub struct LogMessage {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub message: String,
    pub level: i32,
    pub context: Option<Value>,
}

impl TryFrom<LogEntry> for LogMessage {
    type Error = Box<dyn std::error::Error + Send + Sync + 'static>;

    fn try_from(log: LogEntry) -> Result<Self, Self::Error> {
        let naive_dt = NaiveDateTime::from_timestamp_millis(log.time_nano)
            .ok_or("Invalid timestamp")?; // TODO: Unsure if this is correct; is it actually using nano timestamps?
        let dt = DateTime::<Utc>::from_utc(naive_dt, Utc);

        // TODO: add support for partial log entries

        // attempt to parse the log line as JSON
        let log = match serde_json::from_slice::<Value>(&log.line) {
            Ok(json) => {
                let json = json.as_object()
                    .ok_or("Invalid or unexpected format")?;

                let empty = Value::String(String::from(""));
                let message = json.get("message")
                    .unwrap_or(&empty)
                    .as_str()
                    .ok_or("Invalid message")?;

                let level = json.get("level")
                    .unwrap_or(&Value::Number(Number::from(3)))
                    .as_i64()
                    .ok_or("Invalid level")?;
                
                let context = json.iter()
                    .filter(|(k, _)| *k != "message" && *k != "level")
                    .map(|(k, v)| (k.clone(), v.clone()))
                    // insert the source into the context
                    .chain(vec![("source".to_string(), Value::String(log.source))].into_iter())
                    .collect::<Value>();

                Self {
                    timestamp: dt,
                    message: message.to_string(),
                    level: level as i32,
                    context: Some(context),
                }
            },
            Err(_) => { // if it fails to parse as json, treat as string
                let message = String::from_utf8(log.line)
                    .map_err(|_| "Invalid UTF-8")?;
                
                let context = serde_json::json!({
                    "source": log.source,
                });

                Self {
                    timestamp: dt,
                    message,
                    level: 3,
                    context: Some(context),
                }
            }
        };

        Ok(log)
    }
}


#[cfg(test)]
mod tests {
    use docker_protobuf::LogEntry;

    use super::*;
    use std::convert::TryInto;

    #[test]
    fn test_into_log_message() {
        let expected_time_nano = 1620000000000;
        let log = LogEntry {
            time_nano: expected_time_nano,
            line: r#"{"message":"test","level":2,"another_field": 4}"#
                .as_bytes()
                .to_vec(),
            partial: false,
            partial_log_metadata: None,
            source: "container-id".to_string(),
        };

        let log: LogMessage = log
            .try_into()
            .unwrap();

        let naive_dt = NaiveDateTime::from_timestamp_millis(expected_time_nano)
            .unwrap();
        assert_eq!(log.timestamp, chrono::DateTime::<chrono::Utc>::from_utc(naive_dt, Utc));
        assert_eq!(log.message, "test");
        assert_eq!(log.level, 2);

        assert_eq!(log.context, Some(serde_json::json!({
            "another_field": 4,
            "source": "container-id",
        })));
    }

    #[test]
    fn test_string_log_message() {
        let expected_time_nano = 1620000000000;
        let log = LogEntry {
            time_nano: expected_time_nano,
            line: r#"test"#.as_bytes().to_vec(),
            partial: false,
            partial_log_metadata: None,
            source: "container-id".to_string(),
        };

        let log: LogMessage = log
            .try_into()
            .unwrap();

        let naive_dt = NaiveDateTime::from_timestamp_millis(expected_time_nano)
            .unwrap();
        assert_eq!(log.timestamp, chrono::DateTime::<chrono::Utc>::from_utc(naive_dt, Utc));
        assert_eq!(log.message, "test");
        assert_eq!(log.level, 3);

        assert_eq!(log.context, Some(serde_json::json!({
            "source": "container-id",
        })));
    }
}
