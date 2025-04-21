use std::{error::Error, fmt, str::FromStr, time::Duration};

#[derive(Debug)]
pub struct Config<Ctx: Send + Sync> {
    pub port: i32,
    pub ctx: Ctx,
    pub persist_connection_for: Duration,
}

impl<Ctx: Send + Sync> Config<Ctx> {
    pub fn addr(&self) -> String {
        format!("127.0.0.1:{}", self.port)
    }
}

#[derive(Debug, Clone, Copy)]
pub enum HttpProtocol {
    Http11,
}

impl fmt::Display for HttpProtocol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl FromStr for HttpProtocol {
    type Err = HttpProtocolParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "HTTP/1.1" => Ok(HttpProtocol::Http11),
            _ => Err(HttpProtocolParseError),
        }
    }
}

#[derive(Debug)]
pub struct HttpProtocolParseError;

impl fmt::Display for HttpProtocolParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Unsupported HTTP Protocol")
    }
}

impl Error for HttpProtocolParseError {}
