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

extern crate git2;
extern crate docopt;
extern crate rustc_serialize;

use std::str;
use docopt::Docopt;
use git2::{Repository, Error, Tag, Commit};

#[derive(RustcDecodable)]
struct Args {
    arg_tagname: Option<String>,
    arg_object: Option<String>,
    arg_pattern: Option<String>,
    flag_n: Option<u32>,
    flag_force: bool,
    flag_list: bool,
    flag_delete: Option<String>,
    flag_message: Option<String>,
}

fn run(args: &Args) -> Result<(), Error> {
    let repo = try!(Repository::open("."));

    if let Some(ref name) = args.arg_tagname {
        let target = args.arg_object.as_ref().map(|s| &s[..]).unwrap_or("HEAD");
        let obj = try!(repo.revparse_single(target));

        if let Some(ref message) = args.flag_message {
            let sig = try!(repo.signature());
            try!(repo.tag(name, &obj, &sig, message, args.flag_force));
        } else {
            try!(repo.tag_lightweight(name, &obj, args.flag_force));
        }

    } else if let Some(ref name) = args.flag_delete {
        let obj = try!(repo.revparse_single(name));
        let id = try!(obj.short_id());
        try!(repo.tag_delete(name));
        println!("Deleted tag '{}' (was {})", name,
                 str::from_utf8(&*id).unwrap());

    } else if args.flag_list {
        let pattern = args.arg_pattern.as_ref().map(|s| &s[..]).unwrap_or("*");
        for name in try!(repo.tag_names(Some(pattern))).iter() {
            let name = name.unwrap();
            let obj = try!(repo.revparse_single(name));

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
        println!("");
    }
}

fn print_commit(commit: &Commit, name: &str, args: &Args) {
    print!("{:<16}", name);
    if args.flag_n.is_some() {
        print_list_lines(commit.message(), args);
    } else {
        println!("");
    }
}

fn print_name(name: &str) {
    println!("{}", name);
}

fn print_list_lines(message: Option<&str>, args: &Args) {
    let message = match message { Some(s) => s, None => return };
    let mut lines = message.lines().filter(|l| !l.trim().is_empty());
    if let Some(first) = lines.next() {
        print!("{}", first);
    }
    println!("");

    for line in lines.take(args.flag_n.unwrap_or(0) as usize) {
        print!("    {}", line);
    }
}

fn main() {
    const USAGE: &'static str = "
usage:
    tag [-a] [-f] [-m <msg>] <tagname> [<object>]
    tag -d <tag>
    tag [-n <n>] -l [<pattern>]

Options:
    -n <n>                  specify number of lines from teh annotation to print
    -f, --force             replace an existing tag with the given name
    -l, --list              list tags with names matching the pattern given
    -d, --delete <tag>      delete the tag specified
    -m, --message <msg>     message for a new tag
    -h, --help              show this message
";

    let args = Docopt::new(USAGE).and_then(|d| d.decode())
                                 .unwrap_or_else(|e| e.exit());
    match run(&args) {
        Ok(()) => {}
        Err(e) => println!("error: {}", e),
    }
}
