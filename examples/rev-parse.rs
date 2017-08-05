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

extern crate git2;
extern crate docopt;
#[macro_use]
extern crate serde_derive;

use docopt::Docopt;
use git2::Repository;

#[derive(Deserialize)]
struct Args {
    arg_spec: String,
    flag_git_dir: Option<String>,
}

fn run(args: &Args) -> Result<(), git2::Error> {
    let path = args.flag_git_dir.as_ref().map(|s| &s[..]).unwrap_or(".");
    let repo = try!(Repository::open(path));

    let revspec = try!(repo.revparse(&args.arg_spec));

    if revspec.mode().contains(git2::REVPARSE_SINGLE) {
        println!("{}", revspec.from().unwrap().id());
    } else if revspec.mode().contains(git2::REVPARSE_RANGE) {
        let to = revspec.to().unwrap();
        let from = revspec.from().unwrap();
        println!("{}", to.id());

        if revspec.mode().contains(git2::REVPARSE_MERGE_BASE) {
            let base = try!(repo.merge_base(from.id(), to.id()));
            println!("{}", base);
        }

        println!("^{}", from.id());
    } else {
        return Err(git2::Error::from_str("invalid results from revparse"))
    }
    Ok(())
}

fn main() {
    const USAGE: &'static str = "
usage: rev-parse [options] <spec>

Options:
    --git-dir           directory for the git repository to check
";

    let args = Docopt::new(USAGE).and_then(|d| d.deserialize())
                                 .unwrap_or_else(|e| e.exit());
    match run(&args) {
        Ok(()) => {}
        Err(e) => println!("error: {}", e),
    }
}
