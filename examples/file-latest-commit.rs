#![deny(warnings)]

use clap::Parser;
use git2::{DiffOptions, Repository, Sort};

#[derive(Parser)]
struct Args {
    /// Directory to use as the base directory
    #[arg(short, long, default_value = ".")]
    dir: String,

    /// Path to the file relative to the git repo
    #[arg(short, long)]
    path: String,

    /// Branch to check against, otherwise uses the default
    #[arg(short, long)]
    branch: Option<String>,
}

fn run(args: &Args) -> Result<(), git2::Error> {
    let repo = Repository::open(args.dir.clone())?;

    let mut revwalk = repo.revwalk()?;
    match &args.branch {
        Some(branch) => revwalk.push_ref(&format!("refs/heads/{}", branch))?,
        None => revwalk.push_head()?,
    };
    revwalk.set_sorting(Sort::TIME)?;
    let branch_display = match &args.branch {
        Some(branch) => format!("on branch {}", branch),
        None => "on default branch".to_string(),
    };

    for oid_result in revwalk {
        let oid = oid_result?;
        let commit = repo.find_commit(oid)?;

        let tree = commit.tree()?;

        let parent_tree = if commit.parent_count() > 0 {
            Some(commit.parent(0)?.tree()?)
        } else {
            // Initial commit
            None
        };

        let mut diff_opts = DiffOptions::new();
        diff_opts.pathspec(args.path.clone());

        let diff =
            repo.diff_tree_to_tree(parent_tree.as_ref(), Some(&tree), Some(&mut diff_opts))?;

        if diff.deltas().len() > 0 {
            println!(
                "Most recent commit modifying {} {}: {}",
                args.path, branch_display, oid
            );
            return Ok(());
        }
    }

    println!(
        "Error: no modifying commit found modifying {} {}",
        args.path, branch_display
    );
    Ok(())
}

fn main() {
    let args = Args::parse();
    match run(&args) {
        Ok(()) => {}
        Err(e) => println!("error: {}", e),
    }
}
