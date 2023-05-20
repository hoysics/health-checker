// use health_checker::ThreadPool;
// use super::*;
use async_std::task;
use std::{
    fs,
    // io::{prelude::*, BufReader},
    // net::{TcpListener, TcpStream},
    // thread,
    time::Duration,
};
use async_std::prelude::*;
use async_std::net::TcpListener;
// use async_std::net::TcpStream;
use futures::stream::StreamExt;
use async_std::task::spawn;
use async_std::io::{Read, Write};
use futures::io::Error;
use futures::task::{Context, Poll};
use std::cmp::min;
use std::pin::Pin;
use std::marker::Unpin;

#[async_std::main]
async fn main() {
    // 监听地址: 127.0.0.1:7878
    let listener = TcpListener::bind("127.0.0.1:7878").await.unwrap();
    // let pool = ThreadPool::new(4);

    // for stream in listener.incoming() {
    //     let stream = stream.unwrap();
    //     // thread::spawn(|| {
    //     //     handle_connection(stream);
    //     // });
    //     // pool.execute(|| {
    //     handle_connection(stream).await;
    //     // });
    //     // println!("Connection established!");
    //     // handle_connection(stream);
    // }
    listener.incoming().for_each_concurrent(/*limit*/ None, |tcpstream| async move {
        // let tcpstream=tcpstream.unwrap();
        // handle_connection(tcpstream).await;
        let tcpstream = tcpstream.unwrap();
        spawn(handle_connection(tcpstream));
    }).await;
    println!("Shutting down");
}

// fn handle_connection(mut stream: TcpStream) {
//     let buf_reader = BufReader::new(&stream);
//     let request_line = buf_reader.lines().next().unwrap().unwrap();
//     // let (status_line, filename) = if request_line == "GET / HTTP/1.1" {
//     //     ("HTTP/1.1 200 OK", "hello.html")
//     // } else {
//     //     ("HTTP/1.1 404 NOT FOUND", "404.html")
//     // };
//     let (status_line, filename) = match &request_line[..] {
//         "GET / HTTP/1.1" => ("HTTP/1.1 200 OK", "hello.html"),
//         "GET /sleep HTTP/1.1" => {
//             thread::sleep(Duration::from_secs(5));
//             ("HTTP/1.1 200 OK", "hello.html")
//         }
//         _ => ("HTTP/1.1 404 NOT FOUND", "404.html"),
//     };
//     let contents = fs::read_to_string(filename).unwrap();
//     let length = contents.len();
//     let response = format!("{status_line}\r\nContent-Length: {length}\r\n\r\n{contents}");
//     stream.write_all(response.as_bytes()).unwrap();
// }
async fn handle_connection(mut stream: impl Read+Write+Unpin)  {
    let mut buffer=[0;1024];
    stream.read(&mut buffer).await.unwrap();
    let get = b"GET / HTTP/1.1\r\n";
    let sleep = b"GET /sleep HTTP/1.1\r\n";
    let (status_line, filename) = if buffer.starts_with(get) {
        ("HTTP/1.1 200 OK\r\n\r\n", "hello.html")
    } else if buffer.starts_with(sleep) {
        task::sleep(Duration::from_secs(5)).await;
        ("HTTP/1.1 200 OK\r\n\r\n", "hello.html")
    } else {
        ("HTTP/1.1 404 NOT FOUND\r\n\r\n", "404.html")
    };
    let contents = fs::read_to_string(filename).unwrap();
    let response = format!("{status_line}{contents}");
    stream.write(response.as_bytes()).await.unwrap();
    stream.flush().await.unwrap();
}

//
// fn handle_connection(mut stream: TcpStream) {
//     //读取buf
//     let buf_reader = BufReader::new(&stream);
//     let request_line = buf_reader.lines().next().unwrap().unwrap();
//     //打印http请求
//     // let http_request: Vec<_> = buf_reader
//     //     .lines()
//     //     .map(|result| result.unwrap())
//     //     .take_while(|line| !line.is_empty())
//     //     .collect();
//     // println!("Request: {:#?}", http_request);
//     // let response = "HTTP/1.1 200 OK\r\n\r\n";
//     // stream.write_all(response.as_bytes()).unwrap();
//     if request_line == "GET / HTTP/1.1" {
//         let status_line = "HTTP/1.1 200 OK";
//         let contents = fs::read_to_string("hello.html").unwrap();
//         let length = contents.len();
//         let response = format!("{status_line}\r\nContent-Length: {length}\r\n\r\n{contents}");
//         stream.write_all(response.as_bytes()).unwrap();
//     } else {
//         let status_line = "HTTP/1.1 404 NOT FOUND";
//         let contents = fs::read_to_string("404.html").unwrap();
//         let length = contents.len();
//
//         let response = format!("{status_line}\r\nContent-Length: {length}\r\n\r\n{contents}");
//
//         stream.write_all(response.as_bytes()).unwrap();
//     }
// }

struct MockTcpStream{
    read_data:Vec<u8>,
    write_data:Vec<u8>,
}

impl Read for MockTcpStream {
    // add code here
    fn poll_read(
                self: Pin<&mut Self>,
                cx: &mut Context<'_>,
                buf: &mut [u8],
            ) -> Poll<std::io::Result<usize>> {
        let size:usize=min(self.read_data.len(), buf.len());
        buf[..size].copy_from_slice(&self.read_data[..size]);
        Poll::Ready(Ok(size))
    }
}

impl Write for MockTcpStream {
    fn poll_write(
        mut self: Pin<&mut Self>,
        _: &mut Context,
        buf: &[u8],
    ) -> Poll<Result<usize, Error>> {
        self.write_data = Vec::from(buf);

        Poll::Ready(Ok(buf.len()))
    }

    fn poll_flush(self: Pin<&mut Self>, _: &mut Context) -> Poll<Result<(), Error>> {
        Poll::Ready(Ok(()))
    }

    fn poll_close(self: Pin<&mut Self>, _: &mut Context) -> Poll<Result<(), Error>> {
        Poll::Ready(Ok(()))
    }
}

impl Unpin for MockTcpStream {}


#[async_std::test]
async fn test_handle_connection() {
    let input_bytes = b"GET / HTTP/1.1\r\n";
    let mut contents = vec![0u8; 1024];
    contents[..input_bytes.len()].clone_from_slice(input_bytes);
    let mut stream = MockTcpStream {
        read_data: contents,
        write_data: Vec::new(),
    };

    handle_connection(&mut stream).await;
    let mut buf = [0u8; 1024];
    stream.read(&mut buf).await.unwrap();

    let expected_contents = fs::read_to_string("hello.html").unwrap();
    let expected_response = format!("HTTP/1.1 200 OK\r\n\r\n{}", expected_contents);
    assert!(stream.write_data.starts_with(expected_response.as_bytes()));
}
