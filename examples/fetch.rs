/*
 * libgit2 "fetch" example - shows how to fetch remote data
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
use git2::{Repository, RemoteCallbacks, AutotagOption, FetchOptions};
use std::io::{self, Write};
use std::str;

#[derive(Deserialize)]
struct Args {
    arg_remote: Option<String>,
}

fn run(args: &Args) -> Result<(), git2::Error> {
    let repo = try!(Repository::open("."));
    let remote = args.arg_remote.as_ref().map(|s| &s[..]).unwrap_or("origin");

    // Figure out whether it's a named remote or a URL
    println!("Fetching {} for repo", remote);
    let mut cb = RemoteCallbacks::new();
    let mut remote = try!(repo.find_remote(remote).or_else(|_| {
        repo.remote_anonymous(remote)
    }));
    cb.sideband_progress(|data| {
        print!("remote: {}", str::from_utf8(data).unwrap());
        io::stdout().flush().unwrap();
        true
    });

    // This callback gets called for each remote-tracking branch that gets
    // updated. The message we output depends on whether it's a new one or an
    // update.
    cb.update_tips(|refname, a, b| {
        if a.is_zero() {
            println!("[new]     {:20} {}", b, refname);
        } else {
            println!("[updated] {:10}..{:10} {}", a, b, refname);
        }
        true
    });

    // Here we show processed and total objects in the pack and the amount of
    // received data. Most frontends will probably want to show a percentage and
    // the download rate.
    cb.transfer_progress(|stats| {
        if stats.received_objects() == stats.total_objects() {
            print!("Resolving deltas {}/{}\r", stats.indexed_deltas(),
                   stats.total_deltas());
        } else if stats.total_objects() > 0 {
            print!("Received {}/{} objects ({}) in {} bytes\r",
                   stats.received_objects(),
                   stats.total_objects(),
                   stats.indexed_objects(),
                   stats.received_bytes());
        }
        io::stdout().flush().unwrap();
        true
    });

    // Download the packfile and index it. This function updates the amount of
    // received data and the indexer stats which lets you inform the user about
    // progress.
    let mut fo = FetchOptions::new();
    fo.remote_callbacks(cb);
    try!(remote.download(&[], Some(&mut fo)));

    {
        // If there are local objects (we got a thin pack), then tell the user
        // how many objects we saved from having to cross the network.
        let stats = remote.stats();
        if stats.local_objects() > 0 {
            println!("\rReceived {}/{} objects in {} bytes (used {} local \
                      objects)", stats.indexed_objects(),
                     stats.total_objects(), stats.received_bytes(),
                     stats.local_objects());
        } else {
            println!("\rReceived {}/{} objects in {} bytes",
                     stats.indexed_objects(), stats.total_objects(),
                     stats.received_bytes());
        }
    }

    // Disconnect the underlying connection to prevent from idling.
    remote.disconnect();

    // Update the references in the remote's namespace to point to the right
    // commits. This may be needed even if there was no packfile to download,
    // which can happen e.g. when the branches have been changed but all the
    // needed objects are available locally.
    try!(remote.update_tips(None, true,
                            AutotagOption::Unspecified, None));

    Ok(())
}

fn main() {
    const USAGE: &'static str = "
usage: fetch [options] [<remote>]

Options:
    -h, --help          show this message
";

    let args = Docopt::new(USAGE).and_then(|d| d.deserialize())
                                 .unwrap_or_else(|e| e.exit());
    match run(&args) {
        Ok(()) => {}
        Err(e) => println!("error: {}", e),
    }
}
