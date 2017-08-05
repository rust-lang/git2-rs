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

extern crate git2;
extern crate docopt;
#[macro_use]
extern crate serde_derive;

use docopt::Docopt;
use git2::{Repository, Direction};

#[derive(Deserialize)]
struct Args {
    arg_remote: String,
}

fn run(args: &Args) -> Result<(), git2::Error> {
    let repo = try!(Repository::open("."));
    let remote = &args.arg_remote;
    let mut remote = try!(repo.find_remote(remote).or_else(|_| {
        repo.remote_anonymous(remote)
    }));

    // Connect to the remote and call the printing function for each of the
    // remote references.
    let connection = try!(remote.connect_auth(Direction::Fetch, None, None));

    // Get the list of references on the remote and print out their name next to
    // what they point to.
    for head in try!(connection.list()).iter() {
        println!("{}\t{}", head.oid(), head.name());
    }
    Ok(())
}

fn main() {
    const USAGE: &'static str = "
usage: ls-remote [option] <remote>

Options:
    -h, --help          show this message
";

    let args = Docopt::new(USAGE).and_then(|d| d.deserialize())
                                 .unwrap_or_else(|e| e.exit());
    match run(&args) {
        Ok(()) => {}
        Err(e) => println!("error: {}", e),
    }
}
