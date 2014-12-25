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
extern crate "rustc-serialize" as rustc_serialize;

use docopt::Docopt;
use git2::{Repository, Error, Revwalk, Oid};

#[deriving(RustcDecodable)]
struct Args {
    arg_spec: Vec<String>,
    flag_topo_order: bool,
    flag_date_order: bool,
    flag_reverse: bool,
    flag_not: Vec<String>,
}

fn run(args: &Args) -> Result<(), git2::Error> {
    let repo = try!(Repository::open(&Path::new(".")));
    let mut revwalk = try!(repo.revwalk());

    let mut sort = if args.flag_topo_order {
        git2::SORT_TOPOLOGICAL
    } else if args.flag_date_order {
        git2::SORT_TIME
    } else {
        git2::SORT_NONE
    };
    if args.flag_reverse {
        sort = sort | git2::SORT_REVERSE
    }
    revwalk.set_sorting(sort);
    let specs = args.flag_not.iter().map(|s| (s, true));
    let specs = specs.chain(args.arg_spec.iter().map(|s| (s, false)));
    for (spec, hide) in specs.map(|(a, b)| (a.as_slice(), b)) {
        let not = spec.starts_with("^");
        let id = if not {
            try!(repo.revparse_single(spec.slice_from(1))).id()
        } else if spec.contains("..") {
            let revspec = try!(repo.revparse(spec));
            if revspec.mode().contains(git2::REVPARSE_MERGE_BASE) {
                return Err(Error::from_str("merge bases not implemented"))
            }
            try!(push(&mut revwalk, revspec.from().unwrap().id(), !(hide ^ not)));
            revspec.to().unwrap().id()
        } else {
            try!(repo.revparse_single(spec)).id()
        };
        try!(push(&mut revwalk, id, hide ^ not));
    }

    for id in revwalk {
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
    --topo-order        dox
    --date-order        dox
    --reverse           dox
    --not <spec>        dox
    -h, --help          show this message
";

    let args = Docopt::new(USAGE).and_then(|d| d.decode())
                                 .unwrap_or_else(|e| e.exit());
    match run(&args) {
        Ok(()) => {}
        Err(e) => println!("error: {}", e),
    }
}

