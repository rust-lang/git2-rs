use git2::Repository;

fn main() {
    let reflog = {
        let repo = Repository::init(".").unwrap();
        repo.reflog("HEAD").unwrap()
    };
    let _ = reflog.len();
}
