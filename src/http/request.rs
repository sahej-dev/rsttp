use std::{collections::HashMap, error::Error, fmt, str::FromStr};

use tracing::instrument;

use crate::{config::HttpProtocol, router::path::Path};

use super::header::HttpHeader;

#[derive(Debug, PartialEq)]
pub enum ReqType {
    Get,
    Post,
    Options,
    Connect,
}

impl FromStr for ReqType {
    type Err = ReqTypeParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "GET" => Ok(Self::Get),
            "POST" => Ok(Self::Post),
            "OPTIONS" => Ok(Self::Options),
            "CONNECT" => Ok(Self::Connect),
            _ => Err(ReqTypeParseError),
        }
    }
}

#[derive(Debug)]
pub struct ReqTypeParseError;

impl fmt::Display for ReqTypeParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Unsupported Request Type")
    }
}

impl Error for ReqTypeParseError {}

#[derive(Debug, Clone, PartialEq)]
pub enum AcceptedEncoding {
    Gzip,
}

impl HttpHeader for AcceptedEncoding {
    fn key(&self) -> &str {
        "Accept-Encoding"
    }

    fn val(&self) -> String {
        self.to_string()
    }
}

impl fmt::Display for AcceptedEncoding {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            AcceptedEncoding::Gzip => write!(f, "gzip"),
        }
    }
}

impl FromStr for AcceptedEncoding {
    type Err = MessageEncodingParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "gzip" => Ok(AcceptedEncoding::Gzip),
            _ => Err(MessageEncodingParseError),
        }
    }
}

#[derive(Debug)]
pub struct MessageEncodingParseError;

impl fmt::Display for MessageEncodingParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Accept Encoding Parse Error")
    }
}

impl Error for MessageEncodingParseError {}

#[derive(Debug)]
pub struct Request {
    pub req_type: ReqType,
    pub path: Path,
    pub protocol: HttpProtocol,
    pub headers: HashMap<String, String>,
    pub body: String,
    pub accept_encodings: Vec<AcceptedEncoding>,
}

impl Request {
    #[instrument]
    pub fn new(data: &str) -> Result<Request, String> {
        let split_data: Vec<&str> = data.split("\r\n").collect();

        if split_data.is_empty() {
            return Err(String::from("Empty Request Metadata"));
        }

        let req_info: &str = split_data[0];

        let req_info_split: Vec<&str> = req_info.split(' ').collect();
        if req_info_split.len() != 3 {
            return Err(String::from("Malformed Request Metadata"));
        }

        let req_type: ReqType = ReqType::from_str(req_info_split[0]).map_err(|e| e.to_string())?;
        let req_target: Path = Path::parse(&extract_path_from_req_target(req_info_split[1])?)
            .map_err(|e| e.to_string())?;
        let req_protocol: HttpProtocol =
            HttpProtocol::from_str(req_info_split[2]).map_err(|e| e.to_string())?;

        let mut req_headers: HashMap<String, String> = HashMap::new();

        let mut req_accept_encoding: Vec<AcceptedEncoding> = vec![];

        for item in split_data.iter().take(split_data.len() - 2).skip(1) {
            let header_data: Vec<&str> = item.split(": ").collect();

            if header_data.len() != 2 {
                continue;
            }

            if header_data[0].eq_ignore_ascii_case("accept-encoding") {
                let encodings = header_data[1]
                    .split(",")
                    .map(str::trim)
                    .filter(|e| !e.is_empty());

                for encoding in encodings {
                    if let Ok(e) = AcceptedEncoding::from_str(encoding) {
                        req_accept_encoding.push(e);
                    }
                }
            }

            req_headers.insert(header_data[0].to_lowercase(), String::from(header_data[1]));
        }

        let body_split: Vec<&str> = data.split("\r\n\r\n").collect();
        let req_body: String = if body_split.len() > 1 {
            body_split[1..].join("\r\n\r\n")
        } else {
            String::from("")
        };

        Ok(Request {
            req_type,
            path: req_target,
            protocol: req_protocol,
            headers: req_headers,
            body: req_body,
            accept_encodings: req_accept_encoding,
        })
    }

    pub fn header_val(&self, header_key: &str) -> Option<&String> {
        self.headers.get(header_key.to_lowercase().as_str())
    }

    pub fn has_connection_close_header(&self) -> bool {
        if let Some(val) = self.header_val("Connection") {
            return val == "close";
        }

        false
    }
}

#[derive(PartialEq, Debug)]
enum RequestTargetForms {
    Origin,
    Absolute,
    Authority,
    Asterisk,
}

fn extract_path_from_req_target(req_target: &str) -> Result<String, String> {
    let form: RequestTargetForms = match req_target {
        "*" => RequestTargetForms::Asterisk,
        s if s.starts_with("http") => RequestTargetForms::Absolute,
        s if s.starts_with("/") => RequestTargetForms::Origin,
        s if s.contains(":") && !s.contains("/") => RequestTargetForms::Authority,
        _ => return Err(String::from("Malformed request target form")),
    };

    match form {
        RequestTargetForms::Origin => Ok(String::from(req_target)),
        RequestTargetForms::Absolute | RequestTargetForms::Authority => {
            let min_req_len = match form {
                RequestTargetForms::Absolute => 3,
                RequestTargetForms::Authority => 2,
                RequestTargetForms::Origin => {
                    return Err(String::from("Unsupported execution. Fatal failure."));
                }
                RequestTargetForms::Asterisk => {
                    return Err(String::from("Unsupported execution. Fatal failure."));
                }
            };

            let parts: Vec<&str> = req_target
                .split(" ")
                .take_while(|s| !s.is_empty())
                .collect();

            match parts.len().cmp(&min_req_len) {
                std::cmp::Ordering::Less => Err(String::from("Invalid Request Form Target")),
                std::cmp::Ordering::Equal => Ok(String::from("/")),
                std::cmp::Ordering::Greater => {
                    let mut path_parts: Vec<&str> = vec![""];

                    for part in parts.iter().skip(min_req_len) {
                        path_parts.push(part);
                    }

                    Ok(path_parts.join("/"))
                }
            }
        }
        RequestTargetForms::Asterisk => Ok(String::from("*")),
    }
}
