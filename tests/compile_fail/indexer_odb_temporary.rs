use git2::{Indexer, Odb};
use std::path::Path;

fn main() {
    let indexer = Indexer::new(Some(&Odb::new().unwrap()), Path::new("."), 0, true).unwrap();
    drop(indexer);
}
