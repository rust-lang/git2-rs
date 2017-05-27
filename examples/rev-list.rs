/*
 * libgit2 "rev-list" example - shows how to transform a rev-spec into a list
 * of commit ids
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
use git2::{Repository, Error, Revwalk, Oid};

#[derive(RustcDecodable)]
struct Args {
    arg_spec: Vec<String>,
    flag_topo_order: bool,
    flag_date_order: bool,
    flag_reverse: bool,
    flag_not: Vec<String>,
}

fn run(args: &Args) -> Result<(), git2::Error> {
    let repo = try!(Repository::open("."));
    let mut revwalk = try!(repo.revwalk());

    let base = if args.flag_reverse {git2::SORT_REVERSE} else {git2::SORT_NONE};
    revwalk.set_sorting(base | if args.flag_topo_order {
        git2::SORT_TOPOLOGICAL
    } else if args.flag_date_order {
        git2::SORT_TIME
    } else {
        git2::SORT_NONE
    });

    let specs = args.flag_not.iter().map(|s| (s, true))
                    .chain(args.arg_spec.iter().map(|s| (s, false)))
                    .map(|(spec, hide)| {
        if spec.starts_with('^') {(&spec[1..], !hide)} else {(&spec[..], hide)}
    });
    for (spec, hide) in specs {
        let id = if spec.contains("..") {
            let revspec = try!(repo.revparse(spec));
            if revspec.mode().contains(git2::REVPARSE_MERGE_BASE) {
                return Err(Error::from_str("merge bases not implemented"))
            }
            try!(push(&mut revwalk, revspec.from().unwrap().id(), !hide));
            revspec.to().unwrap().id()
        } else {
            try!(repo.revparse_single(spec)).id()
        };
        try!(push(&mut revwalk, id, hide));
    }

    for id in revwalk {
        let id = try!(id);
        println!("{}", id);
    }
    Ok(())
}

fn push(revwalk: &mut Revwalk, id: Oid, hide: bool) -> Result<(), Error> {
    if hide {revwalk.hide(id)} else {revwalk.push(id)}
}

fn main() {
    const USAGE: &'static str = "
usage: rev-list [options] [--] <spec>...

Options:
    --topo-order        sort commits in topological order
    --date-order        sort commits in date order
    --reverse           sort commits in reverse
    --not <spec>        don't show <spec>
    -h, --help          show this message
";

    let args = Docopt::new(USAGE).and_then(|d| d.decode())
                                 .unwrap_or_else(|e| e.exit());
    match run(&args) {
        Ok(()) => {}
        Err(e) => println!("error: {}", e),
    }
}

