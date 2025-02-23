use std::{collections::HashMap, net::TcpStream};

use anyhow::bail;

#[derive(Debug, Clone)]
pub struct RequestLine {
    pub method: String,
    pub target: String,
    pub version: String,
}

#[derive(Debug, Clone)]
pub struct Request {
    pub request_line: RequestLine,
    pub headers: HashMap<String, Vec<String>>,
    pub body: Vec<u8>,
}
impl Request {
    pub fn new(stream: &TcpStream) -> anyhow::Result<Self> {
        use std::io::{BufRead, BufReader, Read};
        let mut buf_reader = BufReader::new(stream);
        //read start-line into struct
        let mut start_line = String::new();
        buf_reader.read_line(&mut start_line)?;
        let start_line = start_line.trim();
        let parts: Vec<&str> = start_line.split_whitespace().collect();
        if parts.len() != 3 {
            bail!("Invalid request line");
        }
        let request_line = RequestLine {
            method: parts[0].to_string(),
            target: parts[1].to_string(),
            version: parts[2].to_string(),
        };

        let mut request = Request {
            request_line,
            headers: HashMap::new(),
            body: Vec::new(),
        };
        // read each header field line into hash table by field name until empty line
        let mut line = String::new();
        loop {
            line.clear();
            buf_reader.read_line(&mut line)?;
            let line = line.trim();
            if line.is_empty() {
                break;
            }
            let header = line.split_once(":").unwrap();
            let (header, values) = (header.0.trim(), header.1.trim());
            request
                .headers
                .entry(header.to_string())
                // .and_modify(|values| values.push(header[1].to_string()))
                .or_insert(
                    values
                        .split(",")
                        .map(|value| value.trim().to_string())
                        .collect(),
                );
        }
        // use parsed data to determine if body is expected
        if let Some(content_length) = request.headers.get("Content-Length") {
            // if message expected
            println!("content-length: {:#?}", content_length);
            if let Ok(content_length) = content_length[0].parse::<usize>() {
                // read body until amounts of octets equal to content-length header or connection is
                // closed
                let mut buffer: Vec<u8> = vec![0; content_length];
                buf_reader.read_exact(&mut buffer)?;
                request.body = buffer;
            }
        }

        // println!("request: {:#?}", String::from_utf8(request.body.clone())?);
        Ok(request)
    }
}

// Response
//

pub struct Response {
    pub status_line: StatusLine,
    pub headers: HashMap<String, Vec<String>>,
    pub body: Vec<u8>,
}

pub struct StatusLine {
    pub version: String,
    pub status_code: u16,
    pub reason_phrase: Option<String>,
}

impl StatusLine {
    pub fn to_bytes(&self) -> Vec<u8> {
        let reason_phrase = self.reason_phrase.clone().unwrap_or("".to_string());
        format!("{} {} {}", self.version, self.status_code, reason_phrase)
            .as_bytes()
            .to_vec()
    }
}

impl Response {
    pub fn new() -> Self {
        let mut headers = HashMap::new();
        headers.insert("Server".to_string(), vec!["faras".to_string()]);
        headers.insert("Content-Type".to_string(), vec!["text/plain".to_string()]);
        Response {
            status_line: StatusLine {
                version: "HTTP/1.1".to_string(),
                status_code: 200,
                reason_phrase: Some("OK".to_string()),
            },
            body: Vec::new(),
            headers,
        }
    }

    pub fn to_bytes(&mut self) -> Vec<u8> {
        // Update Content-Length header
        self.headers.insert(
            "Content-Length".to_string(),
            vec![self.body.len().to_string()],
        );

        // Construct headers string
        let mut response = Vec::new();

        // Add status line
        response.extend(self.status_line.to_bytes());
        response.extend(b"\r\n");

        // Add headers
        for (name, values) in &self.headers {
            response.extend(format!("{}: ", name).as_bytes());
            if values.len() > 1 {
                response.extend(values.join(", ").as_bytes());
            } else {
                response.extend(values[0].as_bytes());
            }
            response.extend(b"\r\n");
        }

        // Add blank line between headers and body
        response.extend(b"\r\n");

        // Add body
        response.extend(&self.body);

        response
    }
}

impl Default for Response {
    fn default() -> Self {
        Self::new()
    }
}
