use std::env;
#[allow(unused_imports)]
use std::io::prelude::*;
use std::net::TcpListener;
use std::{fs, net::TcpStream, thread};

use flate2::write::GzEncoder;
use flate2::Compression;
use http::{Request, Response};

fn main() {
    // You can use print statements as follows for debugging, they'll be visible when running tests.
    println!("Logs from your program will appear here!");

    // Uncomment this block to pass the first stage
    //
    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();

    for stream in listener.incoming() {
        match stream {
            Ok(_stream) => {
                println!("accepted new connection");

                thread::spawn(move || handle_connection(&_stream));
            }
            Err(e) => {
                println!("error: {}", e);
            }
        }
    }
}

fn handle_connection(mut stream: &TcpStream) {
    let supported_encodings = ["gzip"];
    let request = Request::new(stream).unwrap();
    println!("request: {request:#?}");
    let mut response = Response::new();
    let path = request.request_line.target;

    let binding = vec![String::from("")].clone();
    let client_encodings = request.headers.get("Accept-Encoding").unwrap_or(&binding);
    let client_encodings = client_encodings
        .iter()
        .filter(|encoding| supported_encodings.contains(&encoding.as_str()))
        .collect::<Vec<_>>();

    let method = request.request_line.method;
    if method == "GET" {
        if path == "/" {
            response.body = b"Hello, World!".to_vec();
        } else if path.starts_with("/echo") {
            response.body = path.trim_start_matches("/echo/").as_bytes().to_vec();
        } else if path.to_lowercase() == "/user-agent" {
            response.body = request
                .headers
                .get("User-Agent")
                .unwrap()
                .join(", ")
                .as_bytes()
                .to_vec();
        } else if path.to_lowercase() == "/ip" {
            response.body = stream
                .peer_addr()
                .unwrap()
                .ip()
                .to_string()
                .as_bytes()
                .to_vec();
        } else if path.starts_with("/files") {
            response.headers.insert(
                "Content-Type".to_string(),
                vec!["application/octet-stream".to_string()],
            );
            let argv = env::args().collect::<Vec<String>>();
            // if argv[1] == "--directory"{
            // }
            let dir = argv[2].clone();

            if let Ok(content) = fs::read(format!("{dir}{}", &path.trim_start_matches("/files/"))) {
                response.body = content
            } else {
                response.status_line.status_code = 404;
                response.status_line.reason_phrase = Some("Not Found".to_string());
                response.body = b"Not Found".to_vec();
            }
        } else {
            response.status_line.status_code = 404;
            response.status_line.reason_phrase = Some("Not Found".to_string());
            response.body = b"Not Found".to_vec();
        }
    } else if method == "POST" {
        if path.starts_with("/files") {
            let file_name = path.trim_start_matches("/files/");
            // create file in /tmp with name from path
            let mut file = fs::File::create(format!("/tmp/{}", file_name)).unwrap();
            // write request body to file
            let written = file.write_all(&request.body);
            if written.is_ok() {
                response.status_line.status_code = 201;
                response.status_line.reason_phrase = Some("Created".to_string());
            }
        }
    } else {
        response.status_line.status_code = 405;
        response.status_line.reason_phrase = Some("Method Not Allowed".to_string());
        response.body = b"Method Not Allowed".to_vec();
    }

    if !client_encodings.is_empty() {
        response.headers.insert(
            "Content-Encoding".to_string(),
            vec![client_encodings[0].clone()],
        );

        // compress in gzip
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        let _ = encoder.write_all(response.body.as_slice());
        response.body = encoder.finish().unwrap();

        // decode to check TODO: make this a test
        // let mut d = GzDecoder::new(response.body.as_slice());
        // let mut s = String::new();
        // d.read_to_string(&mut s).unwrap();
        // println!("{}", s);
    }

    let response = response.to_bytes();
    stream.write_all(&response).unwrap();
}
