/*
 * libgit2 "rev-parse" example - shows how to parse revspecs
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
use git2::Repository;

#[derive(Parser)]
struct Args {
    #[structopt(name = "spec")]
    arg_spec: String,
    #[structopt(name = "dir", long = "git-dir")]
    /// directory of the git repository to check
    flag_git_dir: Option<String>,
}

fn run(args: &Args) -> Result<(), git2::Error> {
    let path = args.flag_git_dir.as_ref().map(|s| &s[..]).unwrap_or(".");
    let repo = Repository::open(path)?;

    let revspec = repo.revparse(&args.arg_spec)?;

    if revspec.mode().contains(git2::RevparseMode::SINGLE) {
        println!("{}", revspec.from().unwrap().id());
    } else if revspec.mode().contains(git2::RevparseMode::RANGE) {
        let to = revspec.to().unwrap();
        let from = revspec.from().unwrap();
        println!("{}", to.id());

        if revspec.mode().contains(git2::RevparseMode::MERGE_BASE) {
            let base = repo.merge_base(from.id(), to.id())?;
            println!("{}", base);
        }

        println!("^{}", from.id());
    } else {
        return Err(git2::Error::from_str("invalid results from revparse"));
    }
    Ok(())
}

fn main() {
    let args = Args::parse();
    match run(&args) {
        Ok(()) => {}
        Err(e) => println!("error: {}", e),
    }
}
