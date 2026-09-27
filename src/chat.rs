use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};

use crate::rag::RagIndex;

const HTML: &str = include_str!("chat.html");

pub fn run_chatbox() {
    let kb = RagIndex::load();
    println!(
        "Indexed {} chunks from {}",
        kb.chunks.len(),
        if kb.source.is_empty() {
            "(no prospectus file found)"
        } else {
            kb.source.as_str()
        }
    );

    let listener = TcpListener::bind("127.0.0.1:7879").expect("Could not bind http://127.0.0.1:7879");
    println!("RAG prospectus chatbox: http://127.0.0.1:7879");
    let _ = open_browser("http://127.0.0.1:7879");

    for stream in listener.incoming() {
        match stream {
            Ok(s) => {
                if let Err(e) = handle_client(s, &kb) {
                    eprintln!("Request error: {e}");
                }
            }
            Err(e) => eprintln!("Connection error: {e}"),
        }
    }
}

pub fn run_terminal_chat() {
    let kb = RagIndex::load();
    println!("Indexed {} chunks. Type quit to exit.\n", kb.chunks.len());
    loop {
        let line = read_line("You: ");
        if line.is_empty() {
            continue;
        }
        let lower = line.to_lowercase();
        if lower == "quit" || lower == "exit" {
            break;
        }
        println!("Assistant:\n{}\n", kb.answer(&line));
    }
}

fn handle_client(mut stream: TcpStream, kb: &RagIndex) -> std::io::Result<()> {
    let mut buf = [0u8; 32_768];
    let n = stream.read(&mut buf)?;
    if n == 0 {
        return Ok(());
    }
    let req = String::from_utf8_lossy(&buf[..n]);
    let request_line = req.split("\r\n").next().unwrap_or("");
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or("GET");
    let path = parts.next().unwrap_or("/");

    if method == "GET" && (path == "/" || path.starts_with("/index")) {
        return write_response(&mut stream, "200 OK", "text/html; charset=utf-8", HTML.as_bytes());
    }

    if method == "POST" && path.starts_with("/ask") {
        let body = req.split("\r\n\r\n").nth(1).unwrap_or("").trim_end_matches('\0');
        let question = json_string(body, "question").unwrap_or_default();
        let answer = kb.answer(&question);
        let payload = format!("{{\"answer\":\"{}\"}}", json_escape(&answer));
        return write_response(
            &mut stream,
            "200 OK",
            "application/json; charset=utf-8",
            payload.as_bytes(),
        );
    }

    write_response(&mut stream, "404 Not Found", "text/plain; charset=utf-8", b"Not found")
}

fn write_response(stream: &mut TcpStream, status: &str, content_type: &str, body: &[u8]) -> std::io::Result<()> {
    let header = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nAccess-Control-Allow-Origin: *\r\nConnection: close\r\n\r\n",
        body.len()
    );
    stream.write_all(header.as_bytes())?;
    stream.write_all(body)?;
    stream.flush()
}

fn json_string(body: &str, key: &str) -> Option<String> {
    let pat = format!("\"{key}\"");
    let i = body.find(&pat)?;
    let after = &body[i + pat.len()..];
    let colon = after.find(':')?;
    let rest = after[colon + 1..].trim_start();
    if !rest.starts_with('"') {
        let end = rest.find([',', '}']).unwrap_or(rest.len());
        return Some(rest[..end].trim().trim_matches('"').to_string());
    }
    let mut out = String::new();
    let mut chars = rest[1..].chars();
    while let Some(c) = chars.next() {
        match c {
            '\\' => {
                if let Some(n) = chars.next() {
                    out.push(match n {
                        'n' => '\n',
                        't' => '\t',
                        '"' => '"',
                        '\\' => '\\',
                        _ => n,
                    });
                }
            }
            '"' => break,
            _ => out.push(c),
        }
    }
    Some(out)
}

fn json_escape(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "")
}

fn open_browser(url: &str) -> std::io::Result<()> {
    std::process::Command::new("cmd").args(["/C", "start", "", url]).spawn()?;
    Ok(())
}

fn read_line(prompt: &str) -> String {
    use std::io::{self, Write};
    print!("{prompt}");
    let _ = io::stdout().flush();
    let mut s = String::new();
    io::stdin().read_line(&mut s).ok();
    s.trim().to_string()
}
