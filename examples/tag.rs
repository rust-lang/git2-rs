/*
 * libgit2 "tag" example - shows how to list, create and delete tags
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
use git2::{Commit, Error, Repository, Tag};
use std::str;

#[derive(Parser)]
struct Args {
    arg_tagname: Option<String>,
    arg_object: Option<String>,
    arg_pattern: Option<String>,
    #[structopt(name = "n", short)]
    /// specify number of lines from the annotation to print
    flag_n: Option<u32>,
    #[structopt(name = "force", short, long)]
    /// replace an existing tag with the given name
    flag_force: bool,
    #[structopt(name = "list", short, long)]
    /// list tags with names matching the pattern given
    flag_list: bool,
    #[structopt(name = "tag", short, long = "delete")]
    /// delete the tag specified
    flag_delete: Option<String>,
    #[structopt(name = "msg", short, long = "message")]
    /// message for a new tag
    flag_message: Option<String>,
}

fn run(args: &Args) -> Result<(), Error> {
    let repo = Repository::open(".")?;

    if let Some(ref name) = args.arg_tagname {
        let target = args.arg_object.as_ref().map(|s| &s[..]).unwrap_or("HEAD");
        let obj = repo.revparse_single(target)?;

        if let Some(ref message) = args.flag_message {
            let sig = repo.signature()?;
            repo.tag(name, &obj, &sig, message, args.flag_force)?;
        } else {
            repo.tag_lightweight(name, &obj, args.flag_force)?;
        }
    } else if let Some(ref name) = args.flag_delete {
        let obj = repo.revparse_single(name)?;
        let id = obj.short_id()?;
        repo.tag_delete(name)?;
        println!(
            "Deleted tag '{}' (was {})",
            name,
            str::from_utf8(&*id).unwrap()
        );
    } else if args.flag_list {
        let pattern = args.arg_pattern.as_ref().map(|s| &s[..]).unwrap_or("*");
        for name in repo.tag_names(Some(pattern))?.iter() {
            let name = name.unwrap();
            let obj = repo.revparse_single(name)?;

            if let Some(tag) = obj.as_tag() {
                print_tag(tag, args);
            } else if let Some(commit) = obj.as_commit() {
                print_commit(commit, name, args);
            } else {
                print_name(name);
            }
        }
    }
    Ok(())
}

fn print_tag(tag: &Tag, args: &Args) {
    print!("{:<16}", tag.name().unwrap());
    if args.flag_n.is_some() {
        print_list_lines(tag.message(), args);
    } else {
        println!();
    }
}

fn print_commit(commit: &Commit, name: &str, args: &Args) {
    print!("{:<16}", name);
    if args.flag_n.is_some() {
        print_list_lines(commit.message(), args);
    } else {
        println!();
    }
}

fn print_name(name: &str) {
    println!("{}", name);
}

fn print_list_lines(message: Option<&str>, args: &Args) {
    let message = match message {
        Some(s) => s,
        None => return,
    };
    let mut lines = message.lines().filter(|l| !l.trim().is_empty());
    if let Some(first) = lines.next() {
        print!("{}", first);
    }
    println!();

    for line in lines.take(args.flag_n.unwrap_or(0) as usize) {
        print!("    {}", line);
    }
}

fn main() {
    let args = Args::parse();
    match run(&args) {
        Ok(()) => {}
        Err(e) => println!("error: {}", e),
    }
}
