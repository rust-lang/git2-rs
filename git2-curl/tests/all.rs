//! A simple test to verify that git2-curl can communicate to git over HTTP.

use std::fs::File;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::Path;
use std::process::{Command, Stdio};
use tempfile::TempDir;

const PORT: u16 = 7848;

/// This is a very bare-bones HTTP server, enough to run git-http-backend as a CGI.
fn handle_client(stream: TcpStream, working_dir: &Path) {
    let mut buf = BufReader::new(stream);
    let mut line = String::new();
    if buf.read_line(&mut line).unwrap() == 0 {
        panic!("unexpected termination");
    }
    // Read the "METHOD path HTTP/1.1" line.
    let mut parts = line.split_ascii_whitespace();
    let method = parts.next().unwrap();
    let path = parts.next().unwrap();
    let (path, query) = path.split_once('?').unwrap_or_else(|| (path, ""));
    let mut content_length = 0;
    let mut content_type = String::new();
    // Read headers.
    loop {
        let mut header = String::new();
        if buf.read_line(&mut header).unwrap() == 0 {
            panic!("unexpected header");
        }
        if header == "\r\n" {
            break;
        }
        let (name, value) = header.split_once(':').unwrap();
        let name = name.to_ascii_lowercase();
        match name.as_str() {
            "content-length" => content_length = value.trim().parse().unwrap_or(0),
            "content-type" => content_type = value.trim().to_owned(),
            _ => {}
        }
    }
    let mut body = vec![0u8; content_length];
    if content_length > 0 {
        buf.read_exact(&mut body).unwrap();
    }

    let mut cgi_env = vec![
        ("GIT_PROJECT_ROOT", "."),
        ("GIT_HTTP_EXPORT_ALL", "1"),
        ("REQUEST_METHOD", method),
        ("PATH_INFO", path),
        ("QUERY_STRING", query),
        ("CONTENT_TYPE", &content_type),
        ("REMOTE_USER", ""),
        ("REMOTE_ADDR", "127.0.0.1"),
        ("AUTH_TYPE", ""),
        ("REMOTE_HOST", ""),
        ("SERVER_PROTOCOL", "HTTP/1.1"),
        ("REQUEST_URI", path),
    ];

    let cl = content_length.to_string();
    cgi_env.push(("CONTENT_LENGTH", cl.as_str()));

    // Spawn git-http-backend
    let mut cmd = Command::new("git");
    cmd.current_dir(working_dir);
    cmd.arg("http-backend");
    for (k, v) in &cgi_env {
        cmd.env(k, v);
    }
    cmd.stdin(Stdio::piped()).stdout(Stdio::piped());

    let mut child = cmd.spawn().expect("failed to spawn git-http-backend");

    if content_length > 0 {
        child.stdin.as_mut().unwrap().write_all(&body).unwrap();
    }

    let mut cgi_output = Vec::new();
    child
        .stdout
        .as_mut()
        .unwrap()
        .read_to_end(&mut cgi_output)
        .unwrap();

    // Split CGI output into headers and body.
    let index = cgi_output
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .unwrap_or(0);
    let (headers, body) = (&cgi_output[..index], &cgi_output[index + 4..]);
    let content_length = body.len();

    // Write HTTP response
    let mut stream = buf.into_inner();
    stream
        .write_all(
            &format!(
                "HTTP/1.1 200 ok\r\n\
                Connection: close\r\n\
                Content-Length: {content_length}\r\n"
            )
            .into_bytes(),
        )
        .unwrap();
    stream.write_all(headers).unwrap();
    stream.write_all(b"\r\n\r\n").unwrap();
    stream.write_all(body).unwrap();
    stream.flush().unwrap();
}

fn main() {
    let td = TempDir::new().unwrap();
    let td_path = td.path().to_owned();

    // Spin up a server for git-http-backend
    std::thread::spawn(move || {
        let listener = TcpListener::bind(("localhost", PORT)).unwrap();
        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    let td_path = td_path.clone();
                    std::thread::spawn(move || handle_client(stream, &td_path));
                }
                Err(e) => {
                    panic!("Connection failed: {}", e);
                }
            }
        }
    });

    unsafe {
        git2_curl::register(curl::easy::Easy::new());
    }

    // Prep a repo with one file called `foo`
    let sig = git2::Signature::now("foo", "bar").unwrap();
    let r1 = git2::Repository::init(td.path()).unwrap();
    File::create(&td.path().join(".git").join("git-daemon-export-ok")).unwrap();
    {
        let mut index = r1.index().unwrap();
        File::create(&td.path().join("foo")).unwrap();
        index.add_path(Path::new("foo")).unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        r1.commit(
            Some("HEAD"),
            &sig,
            &sig,
            "test",
            &r1.find_tree(tree_id).unwrap(),
            &[],
        )
        .unwrap();
    }

    // Clone through the git-http-backend
    let td2 = TempDir::new().unwrap();
    let r = git2::Repository::clone(&format!("http://localhost:{}", PORT), td2.path()).unwrap();
    assert!(File::open(&td2.path().join("foo")).is_ok());
    {
        File::create(&td.path().join("bar")).unwrap();
        let mut index = r1.index().unwrap();
        index.add_path(&Path::new("bar")).unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let parent = r1.head().ok().and_then(|h| h.target()).unwrap();
        let parent = r1.find_commit(parent).unwrap();
        r1.commit(
            Some("HEAD"),
            &sig,
            &sig,
            "test",
            &r1.find_tree(tree_id).unwrap(),
            &[&parent],
        )
        .unwrap();
    }

    let mut remote = r.find_remote("origin").unwrap();
    remote
        .fetch(&["refs/heads/*:refs/heads/*"], None, None)
        .unwrap();
    let b = r.find_branch("master", git2::BranchType::Local).unwrap();
    let id = b.get().target().unwrap();
    let obj = r.find_object(id, None).unwrap();
    r.reset(&obj, git2::ResetType::Hard, None).unwrap();

    assert!(File::open(&td2.path().join("bar")).is_ok());
}
