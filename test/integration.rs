use std::process::{Child, Command, Stdio};
use std::thread::sleep;
use std::time::{Duration, Instant};
use std::net::TcpStream;
use std::io::{Read, Write};

fn start_server() -> Child {
    Command::new("cargo")
        .args(&["run"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("failed to start server")
}

fn send_request(request: &str) -> String {
    let mut stream = TcpStream::connect("127.0.0.1:7878").unwrap();
    stream.write_all(request.as_bytes()).unwrap();
    let mut buffer = String::new();
    stream.read_to_string(&mut buffer).unwrap();
    buffer
}

#[test]
fn root_returns_hello_html() {
    let mut server = start_server();
    sleep(Duration::from_millis(500)); // Give server time to start

    let response = send_request("GET / HTTP/1.1\r\n\r\n");
    assert!(response.contains("HTTP/1.1 200 OK"));
    assert!(response.contains("Hello, world!")); // or whatever is in hello.html

    server.kill().unwrap();
}

#[test]
fn unknown_path_returns_404() {
    let mut server = start_server();
    sleep(Duration::from_millis(500));

    let response = send_request("GET /doesnotexist HTTP/1.1\r\n\r\n");
    assert!(response.contains("HTTP/1.1 404 NOT FOUND"));
    assert!(response.contains("Not Found")); // or whatever is in 404.html

    server.kill().unwrap();
}

#[test]
fn sleep_path_delays_and_returns_hello() {
    let mut server = start_server();
    sleep(Duration::from_millis(500));

    let start = Instant::now();
    let response = send_request("GET /sleep HTTP/1.1\r\n\r\n");
    let elapsed = start.elapsed();

    assert!(response.contains("HTTP/1.1 200 OK"));
    assert!(elapsed >= Duration::from_secs(5));

    server.kill().unwrap();
}
