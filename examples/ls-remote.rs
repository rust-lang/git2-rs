/*
 * libgit2 "ls-remote" example
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
use git2::{Direction, Repository};

#[derive(Parser)]
struct Args {
    #[structopt(name = "remote")]
    arg_remote: String,
}

fn run(args: &Args) -> Result<(), git2::Error> {
    let repo = Repository::open(".")?;
    let remote = &args.arg_remote;
    let mut remote = repo
        .find_remote(remote)
        .or_else(|_| repo.remote_anonymous(remote))?;

    // Connect to the remote and call the printing function for each of the
    // remote references.
    let connection = remote.connect_auth(Direction::Fetch, None, None)?;

    // Get the list of references on the remote and print out their name next to
    // what they point to.
    for head in connection.list()?.iter() {
        println!("{}\t{}", head.oid(), head.name());
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
