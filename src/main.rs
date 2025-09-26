use radio_paje_rust_web::ThreadPool;
use std::{
    env,
    fs,
    io::{BufReader, prelude::*},
    net::{TcpListener, TcpStream}
};

fn main() {
    let port = env::var("PORT").unwrap_or_else(|_| "7878".to_string());
    let addr = format!("127.0.0.1:{}", port);

    let listener = TcpListener::bind(&addr).unwrap();

    println!("Server listening on port {}", port);
    println!("http://{}", addr);

    let pool = ThreadPool::new(20);

    for stream in listener.incoming() {
        let stream = stream.unwrap();

        pool.execute(|| {
            handle_connection(stream);
        });
    }
}

fn handle_connection(mut stream: TcpStream) {
    let buf_reader = BufReader::new(&stream);
    let request_line = match buf_reader.lines().next() {
        Some(Ok(line)) => line,
        _ => {
            // Add logging here, in the future;
            return;
        }
    };

    let (status_line, filename) = match &request_line[..] {
        "GET / HTTP/1.1" => ("HTTP/1.1 200 OK", "home.html"),
        "GET /hello HTTP/1.1" => ("HTTP/1.1 200 OK", "hello.html"),
        _ => ("HTTP/1.1 404 NOT FOUND", "404.html"),
    };

    let contents = fs::read_to_string(filename).unwrap();
    let length = contents.len();

    let response = format!("{status_line}\r\nContent-Length: {length}\r\n\r\n{contents}");

    stream.write_all(response.as_bytes()).unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::{Read, Write};
    use std::net::{TcpListener, TcpStream};
    use std::thread;

    #[test]
    fn handle_connection_renders_home_html() {
        let expected = fs::read_to_string("test_data/home_test.html").unwrap();

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            handle_connection(stream);
        });
        let mut stream = TcpStream::connect(addr).unwrap();
        stream.write_all(b"GET / HTTP/1.1\r\n\r\n").unwrap();
        let mut buffer = String::new();
        stream.read_to_string(&mut buffer).unwrap();

        assert!(buffer.contains("HTTP/1.1 200 OK"));
        assert!(buffer.contains(&expected));
    }

    #[test]
    fn handle_connection_renders_hello_html() {
        let expected = fs::read_to_string("test_data/hello_test.html").unwrap();

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            handle_connection(stream);
        });
        let mut stream = TcpStream::connect(addr).unwrap();
        stream.write_all(b"GET /hello HTTP/1.1\r\n\r\n").unwrap();
        let mut buffer = String::new();
        stream.read_to_string(&mut buffer).unwrap();

        assert!(buffer.contains("HTTP/1.1 200 OK"));
        assert!(buffer.contains(&expected));
    }

    #[test]
    fn handle_connection_returns_404_for_other_paths() {
        let expected = fs::read_to_string("test_data/hello_404_test.html").unwrap();

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            handle_connection(stream);
        });
        let mut stream = TcpStream::connect(addr).unwrap();
        stream
            .write_all(b"GET /doesnotexist HTTP/1.1\r\n\r\n")
            .unwrap();
        let mut buffer = String::new();
        stream.read_to_string(&mut buffer).unwrap();
        assert!(buffer.contains("HTTP/1.1 404 NOT FOUND"));
        assert!(buffer.contains(&expected));
    }
}
