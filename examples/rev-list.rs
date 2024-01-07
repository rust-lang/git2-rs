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

use clap::Parser;
use git2::{Error, Oid, Repository, Revwalk};

#[derive(Parser)]
struct Args {
    #[structopt(name = "topo-order", long)]
    /// sort commits in topological order
    flag_topo_order: bool,
    #[structopt(name = "date-order", long)]
    /// sort commits in date order
    flag_date_order: bool,
    #[structopt(name = "reverse", long)]
    /// sort commits in reverse
    flag_reverse: bool,
    #[structopt(name = "not")]
    /// don't show <spec>
    flag_not: Vec<String>,
    #[structopt(name = "spec", last = true)]
    arg_spec: Vec<String>,
}

fn run(args: &Args) -> Result<(), git2::Error> {
    let repo = Repository::open(".")?;
    let mut revwalk = repo.revwalk()?;

    let base = if args.flag_reverse {
        git2::Sort::REVERSE
    } else {
        git2::Sort::NONE
    };
    revwalk.set_sorting(
        base | if args.flag_topo_order {
            git2::Sort::TOPOLOGICAL
        } else if args.flag_date_order {
            git2::Sort::TIME
        } else {
            git2::Sort::NONE
        },
    )?;

    let specs = args
        .flag_not
        .iter()
        .map(|s| (s, true))
        .chain(args.arg_spec.iter().map(|s| (s, false)))
        .map(|(spec, hide)| {
            if spec.starts_with('^') {
                (&spec[1..], !hide)
            } else {
                (&spec[..], hide)
            }
        });
    for (spec, hide) in specs {
        let id = if spec.contains("..") {
            let revspec = repo.revparse(spec)?;
            if revspec.mode().contains(git2::RevparseMode::MERGE_BASE) {
                return Err(Error::from_str("merge bases not implemented"));
            }
            push(&mut revwalk, revspec.from().unwrap().id(), !hide)?;
            revspec.to().unwrap().id()
        } else {
            repo.revparse_single(spec)?.id()
        };
        push(&mut revwalk, id, hide)?;
    }

    for id in revwalk {
        let id = id?;
        println!("{}", id);
    }
    Ok(())
}

fn push(revwalk: &mut Revwalk, id: Oid, hide: bool) -> Result<(), Error> {
    if hide {
        revwalk.hide(id)
    } else {
        revwalk.push(id)
    }
}

fn main() {
    let args = Args::parse();
    match run(&args) {
        Ok(()) => {}
        Err(e) => println!("error: {}", e),
    }
}
