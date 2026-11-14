#![forbid(unsafe_code)]
use std::io::Write;

fn main() {
    let dir = std::env::temp_dir().join("finalize-twice");
    let repo = git2::Repository::init(&dir).unwrap();
    let odb = repo.odb().unwrap();
    let mut writer = odb.writer(5, git2::ObjectType::Blob).unwrap();
    writer.write_all(b"hello").unwrap();
    writer.finalize().unwrap();
    let _ = writer.finalize();
}
