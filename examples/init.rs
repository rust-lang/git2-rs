/*
 * libgit2 "init" example - shows how to initialize a new repo
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

extern crate git2;
extern crate docopt;
extern crate rustc_serialize;

use docopt::Docopt;
use git2::{Repository, RepositoryInitOptions, RepositoryInitMode, Error};
use std::path::{PathBuf, Path};

#[derive(RustcDecodable)]
struct Args {
    arg_directory: String,
    flag_quiet: bool,
    flag_bare: bool,
    flag_template: Option<String>,
    flag_separate_git_dir: Option<String>,
    flag_initial_commit: bool,
    flag_shared: Option<String>,
}

fn run(args: &Args) -> Result<(), Error> {
    let mut path = PathBuf::from(&args.arg_directory);
    let repo = if !args.flag_bare && args.flag_template.is_none() &&
                  args.flag_shared.is_none() &&
                  args.flag_separate_git_dir.is_none() {
        try!(Repository::init(&path))
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
            opts.mode(try!(parse_shared(&s)));
        }
        try!(Repository::init_opts(&path, &opts))
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
        try!(create_initial_commit(&repo));
        println!("Created empty initial commit");
    }

    Ok(())
}

/// Unlike regular "git init", this example shows how to create an initial empty
/// commit in the repository. This is the helper function that does that.
fn create_initial_commit(repo: &Repository) -> Result<(), Error> {
    // First use the config to initialize a commit signature for the user.
    let sig = try!(repo.signature());

    // Now let's create an empty tree for this commit
    let tree_id = {
        let mut index = try!(repo.index());

        // Outside of this example, you could call index.add_path()
        // here to put actual files into the index. For our purposes, we'll
        // leave it empty for now.

        try!(index.write_tree())
    };

    let tree = try!(repo.find_tree(tree_id));

    // Ready to create the initial commit.
    //
    // Normally creating a commit would involve looking up the current HEAD
    // commit and making that be the parent of the initial commit, but here this
    // is the first commit so there will be no parent.
    try!(repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[]));

    Ok(())
}

fn parse_shared(shared: &str) -> Result<RepositoryInitMode, Error> {
    match shared {
        "false" | "umask" => Ok(git2::REPOSITORY_INIT_SHARED_UMASK),
        "true" | "group" => Ok(git2::REPOSITORY_INIT_SHARED_GROUP),
        "all" | "world" => Ok(git2::REPOSITORY_INIT_SHARED_ALL),
        _ => {
            if shared.starts_with('0') {
                match u32::from_str_radix(&shared[1..], 8).ok() {
                    Some(n) => {
                        Ok(RepositoryInitMode::from_bits_truncate(n))
                    }
                    None => {
                        Err(Error::from_str("invalid octal value for --shared"))
                    }
                }
            } else {
                Err(Error::from_str("unknown value for --shared"))
            }
        }
    }
}

fn main() {
    const USAGE: &'static str = "
usage: init [options] <directory>

Options:
    -q, --quiet                 don't print information to stdout
    --bare                      initialize a new bare repository
    --template <dir>            use <dir> as an initialization template
    --separate-git-dir <dir>    use <dir> as the .git directory
    --initial-commit            create an initial empty commit
    --shared <perms>            permissions to create the repository with
";

    let args = Docopt::new(USAGE).and_then(|d| d.decode())
                                 .unwrap_or_else(|e| e.exit());
    match run(&args) {
        Ok(()) => {}
        Err(e) => println!("error: {}", e),
    }
}
