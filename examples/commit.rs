use git2::{Error, ErrorCode, Repository, Signature};

fn main() -> Result<(), Error> {
    let repository = Repository::open(".")?;

    // We will commit the content of the index
    let mut index = repository.index()?;
    let tree_oid = index.write_tree()?;
    let tree = repository.find_tree(tree_oid)?;

    let parent_commit = match repository.revparse_single("HEAD") {
        Ok(obj) => Some(obj.into_commit().unwrap()),
        // First commit so no parent commit
        Err(e) if e.code() == ErrorCode::NotFound => None,
        Err(e) => return Err(e),
    };

    let mut parents = Vec::new();
    if parent_commit.is_some() {
        parents.push(parent_commit.as_ref().unwrap());
    }

    let signature = Signature::now("username", "username@example.com")?;
    let commit_oid = repository.commit(
        Some("HEAD"),
        &signature,
        &signature,
        "Commit message",
        &tree,
        &parents[..],
    )?;

    let _commit = repository.find_commit(commit_oid)?;

    Ok(())
}
