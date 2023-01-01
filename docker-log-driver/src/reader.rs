use std::{error::Error, fmt, pin::Pin};

use docker_protobuf::LogEntry;
use tokio::io::AsyncReadExt;


#[derive(Debug)]
pub enum ReaderError {
    Io(std::io::Error),
    Protobuf(prost::DecodeError),
}

impl Error for ReaderError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            ReaderError::Io(err) => Some(err),
            ReaderError::Protobuf(err) => Some(err),
        }
    }
}

impl fmt::Display for ReaderError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ReaderError::Io(err) => write!(f, "IO error: {}", err),
            ReaderError::Protobuf(err) => write!(f, "Protobuf error: {}", err),
        }
    }
}

impl From<std::io::Error> for ReaderError {
    fn from(err: std::io::Error) -> Self {
        ReaderError::Io(err)
    }
}

impl From<prost::DecodeError> for ReaderError {
    fn from(err: prost::DecodeError) -> Self {
        ReaderError::Protobuf(err)
    }
}


pub struct Reader<T> {
    reader: Pin<Box<T>>,
}


impl<T: AsyncReadExt> Reader<T> {
    pub fn new(reader: T) -> Self {
        Self {
            reader: Box::pin(reader),
        }
    }

    /// Reads the next LogEntry from the FIFO file.  If EOF is reached,
    /// returns None.  No further reads should be attempted after EOF.
    pub async fn next(&mut self) -> Result<Option<LogEntry>, ReaderError> {
        let mut size: [u8; 4] = [0; 4];

        let maybe_size = self.reader
            .read_exact(&mut size)
            .await;
        
        match maybe_size {
            Ok(_) => (),
            Err(err) => {
                if err.kind() == std::io::ErrorKind::UnexpectedEof {
                    return Ok(None);
                } else {
                    return Err(err.into());
                }
            }
        }
        
        let buffer_size = u32::from_be_bytes(size) as usize;
        let mut buffer = vec![0; buffer_size];

        let maybe_bytes = self.reader
            .read_exact(&mut buffer)
            .await;

        match maybe_bytes {
            Ok(_) => (),
            Err(err) => {
                if err.kind() == std::io::ErrorKind::UnexpectedEof {
                    return Ok(None);
                } else {
                    return Err(err.into());
                }
            }
        };
        
        LogEntry::from_bytes(&buffer)
            .map_err(ReaderError::from)
            .map(Some)
    }
}
