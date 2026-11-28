use git2::{Indexer, Odb};
use std::path::Path;

fn main() {
    let odb = Odb::new().unwrap();
    let indexer = Indexer::new(Some(&odb), Path::new("."), 0, true).unwrap();
    drop(odb);
    drop(indexer);
}
