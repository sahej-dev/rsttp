use std::collections::HashMap;
use std::error::Error;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::str::FromStr;
use std::sync::Arc;
use std::{env, fmt, fs};

use flate2::Compression;
use flate2::write::GzEncoder;

enum HttpResponseCode {
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

#[derive(Debug)]
enum ReqType {
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
struct ReqTypeParseError;

impl fmt::Display for ReqTypeParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Unsupported Request Type")
    }
}

impl Error for ReqTypeParseError {}

#[derive(Debug, PartialEq)]
enum MessageEncoding {
    Gzip,
}

impl Clone for MessageEncoding {
    fn clone(&self) -> Self {
        match self {
            Self::Gzip => Self::Gzip,
        }
    }
}

impl fmt::Display for MessageEncoding {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            MessageEncoding::Gzip => write!(f, "gzip"),
        }
    }
}

impl FromStr for MessageEncoding {
    type Err = MessageEncodingParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "gzip" => Ok(MessageEncoding::Gzip),
            _ => Err(MessageEncodingParseError),
        }
    }
}

#[derive(Debug)]
struct MessageEncodingParseError;

impl fmt::Display for MessageEncodingParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Accept Encoding Parse Error")
    }
}

impl Error for MessageEncodingParseError {}

#[derive(Debug)]
struct Request {
    req_type: ReqType,
    path: String,
    protocol: String,
    headers: HashMap<String, String>,
    body: String,
    accept_encodings: Vec<MessageEncoding>,
}

impl Request {
    fn new(data: &str) -> Result<Request, String> {
        let split_data: Vec<&str> = data.split("\r\n").collect();

        if split_data.is_empty() {
            return Err(String::from("Empty data found"));
        }

        println!("split data: {:?}", split_data);

        let req_info: &str = split_data[0];

        let req_info_split: Vec<&str> = req_info.split(' ').collect();
        if req_info_split.len() != 3 {
            return Err(String::from("Malformed Request Details"));
        }

        let req_type: ReqType = ReqType::from_str(req_info_split[0]).map_err(|e| e.to_string())?;
        let req_target: String = extract_path_from_req_target(req_info_split[1])?;
        let req_protocol: String = String::from(req_info_split[2]);

        let mut req_headers: HashMap<String, String> = HashMap::new();

        let mut req_accept_encoding: Vec<MessageEncoding> = vec![];

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
                    if let Ok(e) = MessageEncoding::from_str(encoding) {
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

    fn header_val(&self, header_key: &str) -> Option<&String> {
        self.headers.get(header_key.to_lowercase().as_str())
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

    println!("Form match is: {:?}", form);

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

enum ContentType {
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

struct Response {
    protocol: String,
    code: HttpResponseCode,
    headers: HashMap<String, String>,
    body: Option<String>,
    content_encoding: Option<MessageEncoding>,
}

impl Response {
    fn empty(code: HttpResponseCode) -> Response {
        Response {
            body: None,
            code,
            content_encoding: None,
            headers: HashMap::new(),
            protocol: String::from("HTTP/1.1"),
        }
    }

    fn new(
        req: &Request,
        code: HttpResponseCode,
        body: Option<String>,
        content_type: ContentType,
    ) -> Response {
        let mut response = Response {
            protocol: String::from("HTTP/1.1"),
            code,
            headers: HashMap::new(),
            body,
            content_encoding: if req.accept_encodings.is_empty() {
                None
            } else {
                Some(req.accept_encodings[0].clone())
            },
        };

        response.set_header(String::from("Content-Type"), content_type.to_string());

        response
    }

    fn set_header(&mut self, key: String, value: String) {
        self.headers.insert(key, value);
    }

    fn write_to<W: std::io::Write>(&self, mut writer: W) -> std::io::Result<()> {
        let (body_bytes, body_len) = match (&self.body, &self.content_encoding) {
            (Some(body), Some(MessageEncoding::Gzip)) => {
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
        if let Some(e) = &self.content_encoding {
            lines.push(format!("Content-Encoding: {}\r\n", e));
        }
        lines.push(format!("Content-Length: {}\r\n", body_len));

        lines.push(String::from("\r\n"));

        writer.write_all(lines.join("").as_bytes())?;
        writer.write_all(&body_bytes)?;

        Ok(())
    }
}

fn respond_success(stream: TcpStream, req: &Request) {
    respond_with_default_msg(stream, req, HttpResponseCode::R200);
}

fn respond(stream: TcpStream, response: Response) {
    match response.write_to(&stream) {
        Ok(_) => {}
        Err(e) => match e.kind() {
            std::io::ErrorKind::BrokenPipe => {
                println!("Client disconnected during response");
            }
            std::io::ErrorKind::ConnectionReset => {
                println!("Connection reset by client");
            }
            _ => {
                eprintln!("ERROR: Failed to write response: {}", e);
            }
        },
    }
}

fn respond_with_default_msg(stream: TcpStream, req: &Request, code: HttpResponseCode) {
    respond(
        stream,
        Response::new(req, code, None, ContentType::TextPlain),
    );
}

fn respond_bad_request(stream: TcpStream, req: &Request) {
    respond(
        stream,
        Response::new(req, HttpResponseCode::R400, None, ContentType::TextPlain),
    );
}

fn handle_connection(mut stream: TcpStream, files_dir: &String) {
    println!("accepted new connection");
    let mut read_data: String = String::new();
    let bytes_read = match stream.read_to_string(&mut read_data) {
        Ok(n) => n,
        Err(_) => {
            println!("EOROROR");
            return respond(stream, Response::empty(HttpResponseCode::R400));
        }
    };

    println!("read {} bytes", bytes_read);
    println!("data:\n{}", read_data);

    match Request::new(read_data.as_str()) {
        Err(_) => respond(stream, Response::empty(HttpResponseCode::R404)),
        Ok(req) => {
            println!("Request: {:?}", req);

            if req.path == "/" {
                respond_success(stream, &req);
            } else if req.path.starts_with("/echo/") {
                if req.path == "/echo/" {
                    respond_bad_request(stream, &req);
                } else {
                    respond(
                        stream,
                        Response::new(
                            &req,
                            HttpResponseCode::R200,
                            Some(String::from(req.path.split_at(6).1)),
                            ContentType::TextPlain,
                        ),
                    );
                }
            } else if req.path == "/user-agent" {
                match req.header_val("User-Agent") {
                    Some(header_val) => respond(
                        stream,
                        Response::new(
                            &req,
                            HttpResponseCode::R200,
                            Some(header_val.clone()),
                            ContentType::TextPlain,
                        ),
                    ),
                    None => respond_bad_request(stream, &req),
                };
            } else if req.path.starts_with("/files/") {
                let file_path: String = format!("{}/{}", files_dir, req.path.split_at(7).1);

                match req.req_type {
                    ReqType::Get => {
                        let file_content = fs::read_to_string(file_path);
                        match file_content {
                            Ok(content) => respond(
                                stream,
                                Response::new(
                                    &req,
                                    HttpResponseCode::R200,
                                    Some(content),
                                    ContentType::ApplicationOctectStream,
                                ),
                            ),
                            Err(_) => {
                                respond_with_default_msg(stream, &req, HttpResponseCode::R404)
                            }
                        };
                    }
                    ReqType::Post => {
                        let _: Result<(), std::io::Error> = fs::create_dir_all(files_dir);

                        let is_file_written: Result<(), std::io::Error> =
                            fs::write(file_path, &req.body);
                        match is_file_written {
                            Ok(_) => respond(
                                stream,
                                Response::new(
                                    &req,
                                    HttpResponseCode::R201,
                                    None,
                                    ContentType::ApplicationOctectStream,
                                ),
                            ),
                            Err(_) => {
                                respond_with_default_msg(stream, &req, HttpResponseCode::R404)
                            }
                        }
                    }
                    ReqType::Options => (),
                    ReqType::Connect => (),
                };
            } else {
                respond_with_default_msg(stream, &req, HttpResponseCode::R404);
            }
        }
    }
}

fn main() {
    let default_file_dir: String = String::from("files/");
    let args: Vec<String> = env::args().collect();

    let files_dir: String = if args.len() >= 3 {
        args[2].clone()
    } else {
        default_file_dir
    };

    let files_dir: Arc<String> = Arc::new(files_dir);

    match TcpListener::bind("127.0.0.1:2000") {
        Ok(listener) => {
            for stream in listener.incoming() {
                match stream {
                    Ok(stream) => {
                        let files_dir = Arc::clone(&files_dir);
                        std::thread::spawn(move || handle_connection(stream, &files_dir));
                        // handle_connecttion(stream);
                    }
                    Err(e) => {
                        println!("error: {}", e);
                    }
                }
            }
        }
        Err(e) => {
            println!("Failed to establish a TCP Listener. Error:\n{}", e);
        }
    };
}
