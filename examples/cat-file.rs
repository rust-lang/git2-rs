/*
 * libgit2 "cat-file" example - shows how to print data from the ODB
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

use std::io::{self, Write};

use docopt::Docopt;
use git2::{Repository, ObjectType, Blob, Commit, Signature, Tag, Tree};

#[derive(Deserialize)]
struct Args {
    arg_object: String,
    flag_t: bool,
    flag_s: bool,
    flag_e: bool,
    flag_p: bool,
    flag_q: bool,
    flag_v: bool,
    flag_git_dir: Option<String>,
}

fn run(args: &Args) -> Result<(), git2::Error> {
    let path = args.flag_git_dir.as_ref().map(|s| &s[..]).unwrap_or(".");
    let repo = try!(Repository::open(path));

    let obj = try!(repo.revparse_single(&args.arg_object));
    if args.flag_v && !args.flag_q {
        println!("{} {}\n--", obj.kind().unwrap().str(), obj.id());
    }

    if args.flag_t {
        println!("{}", obj.kind().unwrap().str());
    } else if args.flag_s || args.flag_e {
        /* ... */
    } else if args.flag_p {
        match obj.kind() {
            Some(ObjectType::Blob) => {
                show_blob(obj.as_blob().unwrap());
            }
            Some(ObjectType::Commit) => {
                show_commit(obj.as_commit().unwrap());
            }
            Some(ObjectType::Tag) => {
                show_tag(obj.as_tag().unwrap());
            }
            Some(ObjectType::Tree) => {
                show_tree(obj.as_tree().unwrap());
            }
            Some(ObjectType::Any) | None => {
                println!("unknown {}", obj.id())
            }
        }
    }
    Ok(())
}

fn show_blob(blob: &Blob) {
    io::stdout().write_all(blob.content()).unwrap();
}

fn show_commit(commit: &Commit) {
    println!("tree {}", commit.tree_id());
    for parent in commit.parent_ids() {
        println!("parent {}", parent);
    }
    show_sig("author", Some(commit.author()));
    show_sig("committer", Some(commit.committer()));
    if let Some(msg) = commit.message() {
        println!("\n{}", msg);
    }
}

fn show_tag(tag: &Tag) {
    println!("object {}", tag.target_id());
    println!("type {}", tag.target_type().unwrap().str());
    println!("tag {}", tag.name().unwrap());
    show_sig("tagger", tag.tagger());

    if let Some(msg) = tag.message() {
        println!("\n{}", msg);
    }
}

fn show_tree(tree: &Tree) {
    for entry in tree.iter() {
        println!("{:06o} {} {}\t{}",
                 entry.filemode(),
                 entry.kind().unwrap().str(),
                 entry.id(),
                 entry.name().unwrap());
    }
}

fn show_sig(header: &str, sig: Option<Signature>) {
    let sig = match sig { Some(s) => s, None => return };
    let offset = sig.when().offset_minutes();
    let (sign, offset) = if offset < 0 {('-', -offset)} else {('+', offset)};
    let (hours, minutes) = (offset / 60, offset % 60);
    println!("{} {} {} {}{:02}{:02}",
             header, sig, sig.when().seconds(), sign, hours, minutes);

}

fn main() {
    const USAGE: &'static str = "
usage: cat-file (-t | -s | -e | -p) [options] <object>

Options:
    -t                  show the object type
    -s                  show the object size
    -e                  suppress all output
    -p                  pretty print the contents of the object
    -q                  suppress output
    -v                  use verbose output
    --git-dir <dir>     use the specified directory as the base directory
    -h, --help          show this message
";

    let args = Docopt::new(USAGE).and_then(|d| d.deserialize())
                                 .unwrap_or_else(|e| e.exit());
    match run(&args) {
        Ok(()) => {}
        Err(e) => println!("error: {}", e),
    }
}
