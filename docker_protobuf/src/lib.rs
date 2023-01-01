use std::io::Cursor;

use prost::Message;


pub mod log {
    pub mod entry {
        include!(concat!(env!("OUT_DIR"), "/log.entry.rs"));
    }
}


pub use log::entry::{
    LogEntry,
    PartialLogEntryMetadata,
};


///message LogEntry {
///	string source = 1;
///	int64 time_nano = 2;
///	bytes line = 3;
///	bool partial = 4;
///	PartialLogEntryMetadata partial_log_metadata = 5;
///}
///
///message PartialLogEntryMetadata {
///	bool last = 1;
///	string id = 2;
///	int32 ordinal = 3;
///}
impl LogEntry {
    pub fn from_bytes(bytes: &[u8]) -> Result<LogEntry, prost::DecodeError> {
        LogEntry::decode(&mut Cursor::new(bytes))
    }

    pub fn to_bytes(&self) -> Result<Vec<u8>, prost::EncodeError> {
        let mut buffer = Vec::new();

        self.encode(&mut buffer)?;
        Ok(buffer)
    }
}
