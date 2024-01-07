/*
 * libgit2 "blame" example - shows how to use the blame API
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
use git2::{BlameOptions, Repository};
use std::io::{BufRead, BufReader};
use std::path::Path;

#[derive(Parser)]
#[allow(non_snake_case)]
struct Args {
    #[structopt(name = "path")]
    arg_path: String,
    #[structopt(name = "spec")]
    arg_spec: Option<String>,
    #[structopt(short = 'M')]
    /// find line moves within and across files
    flag_M: bool,
    #[structopt(short = 'C')]
    /// find line copies within and across files
    flag_C: bool,
    #[structopt(short = 'F')]
    /// follow only the first parent commits
    flag_F: bool,
}

fn run(args: &Args) -> Result<(), git2::Error> {
    let repo = Repository::open(".")?;
    let path = Path::new(&args.arg_path[..]);

    // Prepare our blame options
    let mut opts = BlameOptions::new();
    opts.track_copies_same_commit_moves(args.flag_M)
        .track_copies_same_commit_copies(args.flag_C)
        .first_parent(args.flag_F);

    let mut commit_id = "HEAD".to_string();

    // Parse spec
    if let Some(spec) = args.arg_spec.as_ref() {
        let revspec = repo.revparse(spec)?;

        let (oldest, newest) = if revspec.mode().contains(git2::RevparseMode::SINGLE) {
            (None, revspec.from())
        } else if revspec.mode().contains(git2::RevparseMode::RANGE) {
            (revspec.from(), revspec.to())
        } else {
            (None, None)
        };

        if let Some(commit) = oldest {
            opts.oldest_commit(commit.id());
        }

        if let Some(commit) = newest {
            opts.newest_commit(commit.id());
            if !commit.id().is_zero() {
                commit_id = format!("{}", commit.id())
            }
        }
    }

    let spec = format!("{}:{}", commit_id, path.display());
    let blame = repo.blame_file(path, Some(&mut opts))?;
    let object = repo.revparse_single(&spec[..])?;
    let blob = repo.find_blob(object.id())?;
    let reader = BufReader::new(blob.content());

    for (i, line) in reader.lines().enumerate() {
        if let (Ok(line), Some(hunk)) = (line, blame.get_line(i + 1)) {
            let sig = hunk.final_signature();
            println!(
                "{} {} <{}> {}",
                hunk.final_commit_id(),
                String::from_utf8_lossy(sig.name_bytes()),
                String::from_utf8_lossy(sig.email_bytes()),
                line
            );
        }
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
