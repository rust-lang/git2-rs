/*
 * libgit2 "init" example - shows how to initialize a new repo (also includes how to do an initial commit)
 *
 * Written by the libgit2 contributors
 *
 * To the extent possible under law, the author(s) have dedicated all copyright
 * and related and neighboring rights to this software to the public domain
 * worldwide. This software is distributed without any warranty.
 *
 * You should have received a copy of the CC0 Public Domain Dedication along
 * with this software. If not, see
 * <http://creativecommons.org/publicdomain/zero/1.0/>.
 */

#![deny(warnings)]

use clap::Parser;
use git2::{Error, Repository, RepositoryInitMode, RepositoryInitOptions};
use std::path::{Path, PathBuf};

#[derive(Parser)]
struct Args {
    #[structopt(name = "directory")]
    arg_directory: String,
    #[structopt(name = "quiet", short, long)]
    /// don't print information to stdout
    flag_quiet: bool,
    #[structopt(name = "bare", long)]
    /// initialize a new bare repository
    flag_bare: bool,
    #[structopt(name = "dir", long = "template")]
    /// use <dir> as an initialization template
    flag_template: Option<String>,
    #[structopt(name = "separate-git-dir", long)]
    /// use <dir> as the .git directory
    flag_separate_git_dir: Option<String>,
    #[structopt(name = "initial-commit", long)]
    /// create an initial empty commit
    flag_initial_commit: bool,
    #[structopt(name = "perms", long = "shared")]
    /// permissions to create the repository with
    flag_shared: Option<String>,
}

fn run(args: &Args) -> Result<(), Error> {
    let mut path = PathBuf::from(&args.arg_directory);
    let repo = if !args.flag_bare
        && args.flag_template.is_none()
        && args.flag_shared.is_none()
        && args.flag_separate_git_dir.is_none()
    {
        Repository::init(&path)?
    } else {
        let mut opts = RepositoryInitOptions::new();
        opts.bare(args.flag_bare);
        if let Some(ref s) = args.flag_template {
            opts.template_path(Path::new(s));
        }

        // If you specified a separate git directory, then initialize
        // the repository at that path and use the second path as the
        // working directory of the repository (with a git-link file)
        if let Some(ref s) = args.flag_separate_git_dir {
            opts.workdir_path(&path);
            path = PathBuf::from(s);
        }

        if let Some(ref s) = args.flag_shared {
            opts.mode(parse_shared(s)?);
        }
        Repository::init_opts(&path, &opts)?
    };

    // Print a message to stdout like "git init" does
    if !args.flag_quiet {
        if args.flag_bare || args.flag_separate_git_dir.is_some() {
            path = repo.path().to_path_buf();
        } else {
            path = repo.workdir().unwrap().to_path_buf();
        }
        println!("Initialized empty Git repository in {}", path.display());
    }

    if args.flag_initial_commit {
        create_initial_commit(&repo)?;
        println!("Created empty initial commit");
    }

    Ok(())
}

/// Unlike regular "git init", this example shows how to create an initial empty
/// commit in the repository. This is the helper function that does that.
fn create_initial_commit(repo: &Repository) -> Result<(), Error> {
    // First use the config to initialize a commit signature for the user.
    let sig = repo.signature()?;

    // Now let's create an empty tree for this commit
    let tree_id = {
        let mut index = repo.index()?;

        // Outside of this example, you could call index.add_path()
        // here to put actual files into the index. For our purposes, we'll
        // leave it empty for now.

        index.write_tree()?
    };

    let tree = repo.find_tree(tree_id)?;

    // Ready to create the initial commit.
    //
    // Normally creating a commit would involve looking up the current HEAD
    // commit and making that be the parent of the initial commit, but here this
    // is the first commit so there will be no parent.
    repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])?;

    Ok(())
}

fn parse_shared(shared: &str) -> Result<RepositoryInitMode, Error> {
    match shared {
        "false" | "umask" => Ok(git2::RepositoryInitMode::SHARED_UMASK),
        "true" | "group" => Ok(git2::RepositoryInitMode::SHARED_GROUP),
        "all" | "world" => Ok(git2::RepositoryInitMode::SHARED_ALL),
        _ => {
            if shared.starts_with('0') {
                match u32::from_str_radix(&shared[1..], 8).ok() {
                    Some(n) => Ok(RepositoryInitMode::from_bits_truncate(n)),
                    None => Err(Error::from_str("invalid octal value for --shared")),
                }
            } else {
                Err(Error::from_str("unknown value for --shared"))
            }
        }
    }
}

fn main() {
    let args = Args::parse();
    match run(&args) {
        Ok(()) => {}
        Err(e) => println!("error: {}", e),
    }
}
