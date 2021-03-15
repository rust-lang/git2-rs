use std::fs::File;
use std::path::Path;

use civet::{Config, Server};
use conduit_git_http_backend as git_backend;
use hyper_rustls::HttpsConnector;
use tempfile::TempDir;

const PORT: u16 = 7848;

fn main() {
    unsafe {
        git2_hyper::register(
            hyper::Client::builder()
                .http1_title_case_headers(true)
                .build(HttpsConnector::with_webpki_roots()),
        );
    }

    // Spin up a server for git-http-backend
    let td = TempDir::new().unwrap();
    let mut cfg = Config::new();
    cfg.port(PORT).threads(1);
    let _a = Server::start(cfg, git_backend::Serve(td.path().to_path_buf()));

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
