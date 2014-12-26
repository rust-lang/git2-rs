/*
 * libgit2 "log" example - shows how to walk history and get commit info
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

extern crate "rustc-serialize" as rustc_serialize;
extern crate docopt;
extern crate git2;
extern crate time;

use docopt::Docopt;
use git2::{Repository, Signature, Commit, ObjectType, Time};

#[deriving(RustcDecodable)]
struct Args {
    arg_commit: Vec<String>,
    arg_spec: Vec<String>,
    flag_topo_order: bool,
    flag_date_order: bool,
    flag_reverse: bool,
    flag_author: Option<String>,
    flag_committer: Option<String>,
    flag_grep: Option<String>,
    flag_git_dir: Option<String>,
    flag_skip: Option<uint>,
    flag_max_count: Option<uint>,
    flag_merges: bool,
    flag_no_merges: bool,
    flag_no_min_parents: bool,
    flag_no_max_parents: bool,
    flag_max_parents: Option<uint>,
    flag_min_parents: Option<uint>,
    flag_patch: bool,
}

fn run(args: &Args) -> Result<(), git2::Error> {
    assert!(args.arg_spec.len() == 0, "unimplemented diff specs");
    assert!(!args.flag_patch, "unimplemented patch show");

    let path = args.flag_git_dir.as_ref().map(|s| s.as_slice()).unwrap_or(".");
    let repo = try!(Repository::open(&Path::new(path)));
    let mut revwalk = try!(repo.revwalk());

    // Prepare the revwalk based on CLI parameters
    let base = if args.flag_reverse {git2::SORT_REVERSE} else {git2::SORT_NONE};
    revwalk.set_sorting(base | if args.flag_topo_order {
        git2::SORT_TOPOLOGICAL
    } else if args.flag_date_order {
        git2::SORT_TIME
    } else {
        git2::SORT_NONE
    });
    for commit in args.arg_commit.iter() {
        if commit.starts_with("^") {
            let obj = try!(repo.revparse_single(commit.slice_from(1)));
            try!(revwalk.hide(obj.id()));
            continue
        }
        let revspec = try!(repo.revparse(commit.as_slice()));
        if revspec.mode().contains(git2::REVPARSE_SINGLE) {
            try!(revwalk.push(revspec.from().unwrap().id()));
        } else {
            let from = revspec.from().unwrap().id();
            let to = revspec.to().unwrap().id();
            try!(revwalk.push(to));
            if revspec.mode().contains(git2::REVPARSE_MERGE_BASE) {
                let base = try!(repo.merge_base(from, to));
                let o = try!(repo.find_object(base, Some(ObjectType::Commit)));
                try!(revwalk.push(o.id()));
            }
            try!(revwalk.hide(from));
        }
    }

    if args.arg_commit.len() == 0 {
        try!(revwalk.push_head());
    }

    // Filter our revwalk based on the CLI parameters
    let mut revwalk = revwalk.map(|id| {
        repo.find_commit(id).unwrap()
    }).filter(|commit| {
        let len = commit.parents().len();
        len >= args.min_parents() && match args.max_parents() {
            Some(n) => len < n, None => true
        }
    }).filter(|commit| {
        sig_matches(commit.author(), &args.flag_author) &&
            sig_matches(commit.committer(), &args.flag_committer)
    }).filter(|commit| {
        log_message_matches(commit.message(), &args.flag_grep)
    }).skip(args.flag_skip.unwrap_or(0)).take(args.flag_max_count.unwrap_or(-1));

    // print!
    for commit in revwalk {
        print_commit(&commit);
    }

    Ok(())
}

fn sig_matches(sig: Signature, arg: &Option<String>) -> bool {
    match *arg {
        Some(ref s) => {
            sig.name().map(|n| n.contains(s.as_slice())).unwrap_or(false) ||
                sig.email().map(|n| n.contains(s.as_slice())).unwrap_or(false)
        }
        None => true
    }
}

fn log_message_matches(msg: Option<&str>, grep: &Option<String>) -> bool {
    match (grep, msg) {
        (&None, _) => true,
        (&Some(_), None) => false,
        (&Some(ref s), Some(msg)) => msg.contains(s.as_slice()),
    }
}

fn print_commit(commit: &Commit) {
    println!("commit {}", commit.id());

    if commit.parents().len() > 1 {
        print!("Merge:");
        for id in commit.parent_ids() {
            print!(" {:.8}", id);
        }
        println!("");
    }

    let author = commit.author();
    println!("Author: {} <{}>",
             String::from_utf8_lossy(author.name_bytes()),
             String::from_utf8_lossy(author.email_bytes()));
    print_time(&author.when(), "Date:   ");
    println!("");

    for line in String::from_utf8_lossy(commit.message_bytes()).lines() {
        println!("    {}", line);
    }
    println!("");
}

fn print_time(time: &Time, prefix: &str) {
    let (offset, sign) = match time.offset_minutes() {
        n if n < 0 => (-n, '-'),
        n => (n, '+'),
    };
    let (hours, minutes) = (offset / 60, offset % 60);
    let ts = time::Timespec::new(time.seconds() +
                                 (time.offset_minutes() as i64) * 60, 0);
    let time = time::at(ts);

    println!("{}{} {}{:02}{:02}", prefix,
             time.strftime("%a %b %e %T %Y").unwrap(), sign, hours, minutes);

}

impl Args {
    fn min_parents(&self) -> uint {
        if self.flag_no_min_parents { return 0 }
        self.flag_min_parents.unwrap_or(if self.flag_merges {2} else {0})
    }

    fn max_parents(&self) -> Option<uint> {
        if self.flag_no_max_parents { return None }
        self.flag_max_parents.or(if self.flag_no_merges {Some(1)} else {None})
    }
}

fn main() {
    const USAGE: &'static str = "
usage: log [options] [<commit>..] [--] [<spec>..]

Options:
    --topo-order            sort commits in topological order
    --date-order            sort commits in date order
    --reverse               sort commits in reverse
    --author <user>         author to sort by
    --committer <user>      committer to sort by
    --grep <pat>            pattern to filter commit messages by
    --git-dir <dir>         alternative git directory to use
    --skip <n>              number of commits to skip
    -n, --max-count <n>     maximum number of commits to show
    --merges                only show merge commits
    --no-merges             don't show merge commits
    --no-min-parents        don't require a minimum number of parents
    --no-max-parents        don't require a maximum number of parents
    --max-parents <n>       specify a maximum number of parents for a commit
    --min-parents <n>       specify a minimum number of parents for a commit
    -p, --patch             show commit diff
    -h, --help              show this message
";

    let args = Docopt::new(USAGE).and_then(|d| d.decode())
                                 .unwrap_or_else(|e| e.exit());
    match run(&args) {
        Ok(()) => {}
        Err(e) => println!("error: {}", e),
    }
}
