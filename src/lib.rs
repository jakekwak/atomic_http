use std::{env::current_dir, error::Error, io, path::PathBuf};

#[cfg(feature = "env")]
use std::str::FromStr;

pub use helpers::traits::http_request::RequestUtils;
pub use helpers::traits::http_response::ResponseUtil;
pub use helpers::traits::http_stream::StreamHttp;

pub mod external {
    pub use async_trait;
    #[cfg(feature = "env")]
    pub use dotenv;
    pub use http;
    #[cfg(feature = "response_file")]
    pub use mime_guess;
    pub use tokio;
}

use http::{Request, Response};

#[macro_export]
macro_rules! dev_print {
    ($($rest:tt)*) => {
        if (cfg!(feature = "debug")) {
            std::println!($($rest)*)
        }
    };
}

#[cfg(not(feature = "tokio_rustls"))]
use tokio::net::TcpListener;

use tokio::net::TcpStream;
#[cfg(feature = "tokio_rustls")]
use tokio_rustls::server::TlsStream;

mod helpers;
pub struct Server {
    #[cfg(not(feature = "tokio_rustls"))]
    pub listener: TcpListener,
    pub options: Options,
}

#[derive(Debug, Clone)]
pub struct Options {
    pub no_delay: bool,
    pub read_timeout_miliseconds: u64,
    pub root_path: PathBuf,
    pub read_buffer_size: usize,
    pub read_max_retry: u8,
    pub read_imcomplete_size: usize,
}

impl Options {
    pub fn new() -> Options {
        let mut _options = Options {
            no_delay: true,
            read_timeout_miliseconds: 3000,
            root_path: current_dir().unwrap(),
            read_buffer_size: 4096,
            read_max_retry: 3,
            read_imcomplete_size: 0,
        };

        #[cfg(feature = "env")]
        {
            use std::env;
            if let Ok(data) = env::var("NO_DELAY") {
                // true, false
                if let Ok(data) = data.parse::<bool>() {
                    _options.no_delay = data;
                }
            }
            if let Ok(data) = env::var("READ_TIMEOUT_MILISECONDS") {
                if let Ok(data) = data.parse::<u64>() {
                    _options.read_timeout_miliseconds = data;
                }
            }

            if let Ok(data) = env::var("ROOT_PATH") {
                _options.root_path = PathBuf::from_str(&data).unwrap();
            }

            if let Ok(data) = env::var("READ_BUFFER_SIZE") {
                if let Ok(data) = data.parse::<usize>() {
                    _options.read_buffer_size = data;
                }
            }

            if let Ok(data) = env::var("READ_MAX_RETRY") {
                if let Ok(data) = data.parse::<u8>() {
                    _options.read_max_retry = data;
                }
            }

            if let Ok(data) = env::var("READ_IMCOMPLETE_SIZE") {
                if let Ok(data) = data.parse::<usize>() {
                    _options.read_imcomplete_size = data;
                }
            }
        }

        _options
    }
}

impl Server {
    #[cfg(not(feature = "tokio_rustls"))]
    pub async fn new(address: &str) -> Result<Server, Box<dyn Error>> {
        let listener = TcpListener::bind(address).await?;
        Ok(Server {
            listener,
            options: Options::new(),
        })
    }
    #[cfg(feature = "tokio_rustls")]
    pub async fn new() -> Result<Server, Box<dyn Error>> {
        Ok(Server {
            options: Options::new(),
        })
    }
    #[cfg(not(feature = "tokio_rustls"))]
    pub async fn accept(&self) -> Result<(TcpStream, Options), Box<dyn Error>> {
        use std::time::Duration;

        let (stream, _) = match self.listener.accept().await {
            Ok(data) => data,
            Err(e) => {
                if is_connection_error(&e) {
                    return Err(e.into());
                }
                dev_print!("Accept Error: {:?}", e);

                tokio::time::sleep(Duration::from_secs(1)).await;
                return Err(e.into());
            }
        };
        Ok((stream, self.options.clone()))
    }
    pub async fn parse_request(
        stream: TcpStream,
        options: Options,
    ) -> Result<(Request<Body>, Response<Writer>), Box<dyn Error>> {
        Ok(stream.parse_request(&options).await?)
    }
    #[cfg(feature = "tokio_rustls")]
    pub async fn parse_request(
        stream: TlsStream<TcpStream>,
        options: Options,
    ) -> Result<(Request<Body>, Response<Writer>), Box<dyn Error>> {
        let (stream, _connect) = stream.into_inner();
        Ok(stream.parse_request(&options).await?)
    }
    pub fn set_no_delay(&mut self, no_delay: bool) {
        self.options.no_delay = no_delay;
    }
}

pub struct Body {
    pub bytes: Vec<u8>,
    pub body: String,
    pub len: usize,
}

pub struct Writer {
    pub stream: TcpStream,
    pub body: String,
    pub bytes: Vec<u8>,
    pub use_file: bool,
    pub options: Options,
}

fn is_connection_error(e: &io::Error) -> bool {
    matches!(
        e.kind(),
        io::ErrorKind::ConnectionRefused
            | io::ErrorKind::ConnectionAborted
            | io::ErrorKind::ConnectionReset
    )
}
