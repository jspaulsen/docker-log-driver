use std::path::PathBuf;

use hyper::server::{
    accept::Accept,
    Builder,
    Server,
};
use tokio::{
    net::{
        UnixListener,
        UnixStream,
    },
};
use tokio_stream::wrappers::UnixListenerStream;


pub struct UnixServer {
    path: PathBuf,
}


impl UnixServer {
    const SERVER_PATH: &'static str = "/run/docker/plugins";

    pub fn new<S: Into<String>>(path: S) -> Self {
        Self {
            path: PathBuf::from(path.into())
        }
    }

    pub fn from_fpath<S: Into<String>>(fpath: S) -> Self {
        Self {
            path: PathBuf::from(Self::SERVER_PATH).join(fpath.into()),
        }
    }

    pub fn into_server(self) -> Result<Builder<impl Accept<Conn = UnixStream, Error = std::io::Error>>, std::io::Error> {
        let listener = {
            if self.socket_exists() {
                std::fs::remove_file(&self.path)?;
            }

            UnixListener::bind(self.path)?
        };

        let stream = UnixListenerStream::new(listener);

        Ok(
            Server::builder(
                hyper::server::accept::from_stream(stream)
            )
        )
    }

    fn socket_exists(&self) -> bool {
        self.path.exists()
    }
}
