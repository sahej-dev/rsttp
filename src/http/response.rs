use std::collections::HashMap;
use std::fmt;
use std::io::Write;

use flate2::Compression;
use flate2::write::GzEncoder;

use super::{AcceptedEncoding, Request, header::HttpHeader};
use crate::config::HttpProtocol;

pub enum HttpResponseCode {
    R200,
    R201,
    R400,
    R404,
}

impl HttpResponseCode {
    fn default_message(&self) -> &'static str {
        match self {
            HttpResponseCode::R200 => "OK",
            HttpResponseCode::R201 => "Created",
            HttpResponseCode::R400 => "Bad Request",
            HttpResponseCode::R404 => "Not Found",
        }
    }
}

impl fmt::Display for HttpResponseCode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let text = match self {
            HttpResponseCode::R200 => "200",
            HttpResponseCode::R201 => "201",
            HttpResponseCode::R400 => "400",
            HttpResponseCode::R404 => "404",
        };

        write!(f, "{}", text)
    }
}

pub enum ContentType {
    TextPlain,
    ApplicationOctectStream,
}

impl fmt::Display for ContentType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ContentType::TextPlain => write!(f, "text/plain"),
            ContentType::ApplicationOctectStream => write!(f, "application/octet-stream"),
        }
    }
}

impl HttpHeader for ContentType {
    fn key(&self) -> &str {
        "Content-Type"
    }

    fn val(&self) -> String {
        self.to_string()
    }
}

pub struct Response {
    protocol: HttpProtocol,
    code: HttpResponseCode,
    headers: HashMap<String, String>,
    body: Option<String>,
    content_encoding: Option<AcceptedEncoding>,
    content_type: ContentType,
}

impl Response {
    pub fn success() -> Response {
        Response::default_message(HttpResponseCode::R200)
    }

    pub fn bad_request() -> Response {
        Response::default_message(HttpResponseCode::R400)
    }

    pub fn not_found() -> Response {
        Response::default_message(HttpResponseCode::R404)
    }

    pub fn default_message(code: HttpResponseCode) -> Response {
        Response {
            body: None,
            code,
            content_encoding: None,
            headers: HashMap::new(),
            protocol: HttpProtocol::Http11,
            content_type: ContentType::TextPlain,
        }
    }

    pub fn new(
        req: &Request,
        code: HttpResponseCode,
        body: Option<String>,
        content_type: ContentType,
        protocol: HttpProtocol,
    ) -> Response {
        Response {
            protocol,
            code,
            headers: HashMap::new(),
            body,
            content_type,
            content_encoding: if req.accept_encodings.is_empty() {
                None
            } else {
                Some(req.accept_encodings[0].clone())
            },
        }
    }

    pub fn write_to<W: std::io::Write>(&self, mut writer: W) -> std::io::Result<()> {
        let (body_bytes, body_len) = match (&self.body, &self.content_encoding) {
            (Some(body), Some(AcceptedEncoding::Gzip)) => {
                let mut encoder = GzEncoder::new(Vec::new(), Compression::default());

                if encoder.write_all(body.as_bytes()).is_err() {
                    (body.as_bytes().to_vec(), body.len())
                } else {
                    match encoder.finish() {
                        Ok(cmprsd_bytes) => {
                            let n: usize = cmprsd_bytes.len();
                            println!("compressed_bytes: {:?}", cmprsd_bytes);
                            (cmprsd_bytes, n)
                        }
                        Err(_) => (body.as_bytes().to_vec(), body.len()),
                    }
                }
            }
            (Some(body), _) => (body.as_bytes().to_vec(), body.len()),
            _ => (Vec::new(), 0),
        };

        let mut lines: Vec<String> = vec![format!(
            "HTTP/1.1 {} {}\r\n",
            self.code,
            self.code.default_message()
        )];

        self.headers.iter().for_each(|a| {
            lines.push(format!("{}\r\n", [a.0.as_str(), a.1.as_str()].join(": ")));
        });
        lines.push(self.content_type.in_raw_http_form());
        if let Some(e) = &self.content_encoding {
            lines.push(e.in_raw_http_form());
        }
        lines.push(format!("Content-Length: {}\r\n", body_len));

        lines.push(String::from("\r\n"));

        writer.write_all(lines.join("").as_bytes())?;
        writer.write_all(&body_bytes)?;

        Ok(())
    }
}
