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

use std::io::{self, Write};

use clap::Parser;
use git2::{Blob, Commit, ObjectType, Repository, Signature, Tag, Tree};

#[derive(Parser)]
struct Args {
    #[structopt(name = "object")]
    arg_object: String,
    #[structopt(short = 't')]
    /// show the object type
    flag_t: bool,
    #[structopt(short = 's')]
    /// show the object size
    flag_s: bool,
    #[structopt(short = 'e')]
    /// suppress all output
    flag_e: bool,
    #[structopt(short = 'p')]
    /// pretty print the contents of the object
    flag_p: bool,
    #[structopt(name = "quiet", short, long)]
    /// suppress output
    flag_q: bool,
    #[structopt(name = "verbose", short, long)]
    flag_v: bool,
    #[structopt(name = "dir", long = "git-dir")]
    /// use the specified directory as the base directory
    flag_git_dir: Option<String>,
}

fn run(args: &Args) -> Result<(), git2::Error> {
    let path = args.flag_git_dir.as_ref().map(|s| &s[..]).unwrap_or(".");
    let repo = Repository::open(path)?;

    let obj = repo.revparse_single(&args.arg_object)?;
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
            Some(ObjectType::Any) | None => println!("unknown {}", obj.id()),
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
        println!(
            "{:06o} {} {}\t{}",
            entry.filemode(),
            entry.kind().unwrap().str(),
            entry.id(),
            entry.name().unwrap()
        );
    }
}

fn show_sig(header: &str, sig: Option<Signature>) {
    let sig = match sig {
        Some(s) => s,
        None => return,
    };
    let offset = sig.when().offset_minutes();
    let (sign, offset) = if offset < 0 {
        ('-', -offset)
    } else {
        ('+', offset)
    };
    let (hours, minutes) = (offset / 60, offset % 60);
    println!(
        "{} {} {} {}{:02}{:02}",
        header,
        sig,
        sig.when().seconds(),
        sign,
        hours,
        minutes
    );
}

fn main() {
    let args = Args::parse();
    match run(&args) {
        Ok(()) => {}
        Err(e) => println!("error: {}", e),
    }
}
