/*
 * libgit2 "add" example - shows how to modify the index
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
#![allow(trivial_casts)]

use clap::Parser;
use git2::Repository;
use std::path::Path;

#[derive(Parser)]
struct Args {
    #[structopt(name = "spec")]
    arg_spec: Vec<String>,
    #[structopt(name = "dry_run", short = 'n', long)]
    /// dry run
    flag_dry_run: bool,
    #[structopt(name = "verbose", short, long)]
    /// be verbose
    flag_verbose: bool,
    #[structopt(name = "update", short, long)]
    /// update tracked files
    flag_update: bool,
}

fn run(args: &Args) -> Result<(), git2::Error> {
    let repo = Repository::open(&Path::new("."))?;
    let mut index = repo.index()?;

    let cb = &mut |path: &Path, _matched_spec: &[u8]| -> i32 {
        let status = repo.status_file(path).unwrap();

        let ret = if status.contains(git2::Status::WT_MODIFIED)
            || status.contains(git2::Status::WT_NEW)
        {
            println!("add '{}'", path.display());
            0
        } else {
            1
        };

        if args.flag_dry_run {
            1
        } else {
            ret
        }
    };
    let cb = if args.flag_verbose || args.flag_update {
        Some(cb as &mut git2::IndexMatchedPath)
    } else {
        None
    };

    if args.flag_update {
        index.update_all(args.arg_spec.iter(), cb)?;
    } else {
        index.add_all(args.arg_spec.iter(), git2::IndexAddOption::DEFAULT, cb)?;
    }

    index.write()?;
    Ok(())
}

fn main() {
    let args = Args::parse();
    match run(&args) {
        Ok(()) => {}
        Err(e) => println!("error: {}", e),
    }
}
