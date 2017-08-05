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

extern crate git2;
extern crate docopt;
#[macro_use]
extern crate serde_derive;

use std::path::Path;
use docopt::Docopt;
use git2::Repository;

#[derive(Deserialize)]
struct Args {
    arg_spec: Vec<String>,
    flag_dry_run: bool,
    flag_verbose: bool,
    flag_update: bool,
}

fn run(args: &Args) -> Result<(), git2::Error> {
    let repo = try!(Repository::open(&Path::new(".")));
    let mut index = try!(repo.index());

    let cb = &mut |path: &Path, _matched_spec: &[u8]| -> i32 {
        let status = repo.status_file(path).unwrap();

        let ret = if status.contains(git2::STATUS_WT_MODIFIED) ||
                     status.contains(git2::STATUS_WT_NEW) {
            println!("add '{}'", path.display());
            0
        } else {
            1
        };

        if args.flag_dry_run {1} else {ret}
    };
    let cb = if args.flag_verbose || args.flag_update {
        Some(cb as &mut git2::IndexMatchedPath)
    } else {
        None
    };

    if args.flag_update {
        try!(index.update_all(args.arg_spec.iter(), cb));
    } else {
        try!(index.add_all(args.arg_spec.iter(), git2::ADD_DEFAULT, cb));
    }

    try!(index.write());
    Ok(())
}

fn main() {
    const USAGE: &'static str = "
usage: add [options] [--] [<spec>..]

Options:
    -n, --dry-run       dry run
    -v, --verbose       be verbose
    -u, --update        update tracked files
    -h, --help          show this message
";

    let args = Docopt::new(USAGE).and_then(|d| d.deserialize())
                                 .unwrap_or_else(|e| e.exit());
    match run(&args) {
        Ok(()) => {}
        Err(e) => println!("error: {}", e),
    }
}
