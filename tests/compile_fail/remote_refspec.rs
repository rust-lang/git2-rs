// Regression tests for https://github.com/rust-lang/git2-rs/issues/1286

#[forbid(unsafe_code)]

use git2::Repository;

fn main() {
    let repo = Repository::init("temp_dir").unwrap();
    repo.remote("origin", "https://aaa.com/bbb.git").unwrap();

    let refspec = {
        let remote = repo.find_remote("origin").unwrap();
        remote.get_refspec(0)
    };
    // remote goes out of scope
    // but refspec's lifetime is not bounded to remote    
    
    // removing temp_dir, not necessary to trigger UAF
    let _ = std::fs::remove_dir_all("temp_dir");
    
    // triggers UAF
    let _ = refspec.unwrap().str();
}
