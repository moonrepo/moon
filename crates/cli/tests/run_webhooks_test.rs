mod utils;

use insta::assert_snapshot;
use moon_cache::CacheEngine;
use moon_utils::path::standardize_separators;
use moon_utils::test::{
    create_moon_command, create_sandbox, create_sandbox_with_git, get_assert_output,
};
use predicates::prelude::*;
use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::thread;
use utils::{append_workspace_config, get_path_safe_output};

fn handle_read(mut stream: &TcpStream) {
    let mut buf = [0u8; 4096];
    match stream.read(&mut buf) {
        Ok(_) => {
            let req_str = String::from_utf8_lossy(&buf);
            println!("{}", req_str);
        }
        Err(e) => println!("Unable to read stream: {}", e),
    }
}

fn handle_write(mut stream: TcpStream) {
    let response = b"HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=UTF-8\r\n\r\n<html><body>Hello world</body></html>\r\n";
    match stream.write(response) {
        Ok(_) => println!("Response sent"),
        Err(e) => println!("Failed sending response: {}", e),
    }
}

fn handle_client(stream: TcpStream) {
    // handle_read(&stream);
    // handle_write(stream);
    println!("CLIENT");
}

fn create_localhost_server() -> TcpListener {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();

    println!("CONNECTED {}", listener.local_addr().unwrap());

    tokio::spawn(async {
        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    thread::spawn(|| handle_client(stream));
                }
                Err(e) => {
                    println!("Unable to connect: {}", e);
                }
            }
        }
    });

    listener
}

#[test]
fn sends_webhooks_to_server() {
    let fixture = create_sandbox_with_git("cases");
    let server = create_localhost_server();

    append_workspace_config(
        fixture.path(),
        &format!(
            "notifier:\n  webhookUrl: 'https://127.0.0.1:{}'",
            server.local_addr().unwrap().port()
        ),
    );

    let assert = create_moon_command(fixture.path())
        .arg("run")
        .arg("node:cjs")
        .assert();

    assert.failure();
}
