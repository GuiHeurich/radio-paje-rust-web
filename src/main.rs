use radio_paje_rust_web::ThreadPool;
use std::{
    fs,
    io::{BufReader, prelude::*},
    net::{TcpListener, TcpStream},
    thread,
    time::Duration,
};
fn main() {
    let listener = TcpListener::bind("127.0.0.1:7878").unwrap();

    println!("Server listening on port 7878");
    println!("http://127.0.0.1:7878");

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
    let request_line = buf_reader.lines().next().unwrap().unwrap();

    let (status_line, filename) = match &request_line[..] {
        "GET / HTTP/1.1" => ("HTTP/1.1 200 OK", "hello.html"),
        "GET /sleep HTTP/1.1" => {
            thread::sleep(Duration::from_secs(5));
            ("HTTP/1.1 200 OK", "hello.html")
        }
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
    use std::io::{Read, Write};
    use std::net::{TcpListener, TcpStream};
    use std::thread;
    use std::fs;

    fn setup_test_file(name: &str, contents: &str) {
        fs::write(name, contents).unwrap();
    }

    #[test]
    fn handle_connection_renders_hello_html() {
        setup_test_file("test_data/hello_test.html", "Hello, world!");
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
        assert!(buffer.contains("Hello, world!"));
    }

    #[test]
    fn handle_connection_returns_404_for_other_paths() {
        setup_test_file("test_data/hello_404.html", "Not Found");
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            handle_connection(stream);
        });
        let mut stream = TcpStream::connect(addr).unwrap();
        stream.write_all(b"GET /doesnotexist HTTP/1.1\r\n\r\n").unwrap();
        let mut buffer = String::new();
        stream.read_to_string(&mut buffer).unwrap();
        assert!(buffer.contains("HTTP/1.1 404 NOT FOUND"));
        assert!(buffer.contains("Not Found"));
    }
}
