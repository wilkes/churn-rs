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

extern crate git2;
extern crate docopt;
extern crate rustc_serialize;

use docopt::Docopt;
use git2::{Repository, Error, Revwalk, Oid, Tree, ObjectType};
use std::collections::{HashMap, HashSet};
use std::io::Write;

#[derive(RustcDecodable)]
struct Args {
    flag_topo_order: bool,
    flag_date_order: bool,
    flag_reverse: bool,
}

/*
struct MyCommit {
    oid: Oid,
    summary: String,
    committer: String,
    time: Time,
}

fn git_commit_to_my_commit(mut gcommit: Commit) -> MyCommit {
    MyCommit {
        oid: gcommit.id(),
        summary: gcommit.summary().unwrap_or("").to_string(),
        time: gcommit.time(),
        committer: gcommit.committer().name().unwrap_or("<Unknown>").to_string(),
    }
}
*/

fn get_mut_or_create_with<'a, V, F: FnOnce()->V>(
    map: &'a mut HashMap<String, V>, key: &str, f: F) -> &'a mut V
{
    // It should be possible to avoid the .to_string() call here if the key is
    // already present in the map, but there doesn't seem to be an API that
    // does this yet.
    map.entry(key.to_string()).or_insert_with(f)
}

fn join(base: &str, name: &str) -> String {
    match base {
        "" => name.to_string(),
        _ => base.to_string() + "/" + name
    }
}

struct DirData {
    hashes: HashSet<Oid>,
    files: HashMap<String, HashSet<Oid>>,
    dirs: HashMap<String, DirData>
}

impl DirData {
    fn new() -> DirData {
        DirData {
            hashes: HashSet::new(),
            files: HashMap::new(),
            dirs: HashMap::new()
        }
    }

    fn subdir(&mut self, name: &str) -> &mut DirData {
        get_mut_or_create_with(&mut self.dirs, name, || DirData::new())
    }

    /// Add an entry to `out` for each file in this tree.
    ///
    /// This is like `find . -type f`: directories aren't included, but files
    /// in subdirectories are. And the order of the output is pretty random.
    fn get_all_files(&self, path: &str, out: &mut Vec<(String, usize)>) {
        for (name, hashes) in &self.files {
            let full_path = join(path, name);
            out.push((full_path, hashes.len()));
        }
        for (name, subdir) in &self.dirs {
            let full_path = join(path, name);
            subdir.get_all_files(&full_path, out);
        }
    }

    fn update_for_tree(&mut self, repo: &Repository, tree: &Tree) -> Result<(), Error> {
        for entry in tree.iter() {
            let name = entry.name().unwrap();
            let sha = entry.id();
            match entry.kind() {
                Some(ObjectType::Tree) => {
                    let subdir = self.subdir(name);
                    if subdir.hashes.insert(sha) {
                        let child_object = try!(entry.to_object(repo));
                        let subtree = child_object.as_tree().unwrap();
                        try!(subdir.update_for_tree(repo, subtree));
                    }
                }
                Some(ObjectType::Blob) => {
                    let hashes = get_mut_or_create_with(&mut self.files, name, || HashSet::new());
                    hashes.insert(sha);
                }
                _ => {}
            }
        }
        Ok(())
    }
}

fn config_revwalk(revwalk: &mut Revwalk, args: &Args) -> () {
    let base = if args.flag_reverse {git2::SORT_REVERSE} else {git2::SORT_NONE};
    let sort_type =  if args.flag_topo_order {
                         git2::SORT_TOPOLOGICAL
                     } else if args.flag_date_order {
                         git2::SORT_TIME
                     } else {
                         git2::SORT_NONE
                     };
    revwalk.set_sorting(base | sort_type);
}

const COMMITS_PER_DOT: usize = 1000;

fn run(args: &Args) -> Result<(), git2::Error> {
    let repo = try!(Repository::open("."));
    let mut revwalk = try!(repo.revwalk());
    let spec = "HEAD";

    config_revwalk(&mut revwalk, args);

    let mut root_dir: DirData = DirData::new();

    let id:Oid = try!(repo.revparse_single(spec)).id();
    try!(revwalk.push(id));
    let mut n = 0;
    for id in revwalk {
        let commit = try!(repo.find_commit(id));
        let tree = try!(commit.tree());
        try!(root_dir.update_for_tree(&repo, &tree));
        //let my_commit = git_commit_to_my_commit(commit);
        //println!("{} {}", my_commit.oid, my_commit.summary);

        n += 1;
        if n % COMMITS_PER_DOT == 0 {
            print!(".");
        }
        std::io::stdout().flush().unwrap();
    }
    println!("");

    let mut all_files = vec![];
    root_dir.get_all_files("", &mut all_files);
    all_files.sort();
    for (filename, churn_count) in all_files {
        println!("{}, {}", filename, churn_count);
    }

    Ok(())
}

fn main() {
    const USAGE: &'static str = "
usage: rev-list [options]

Options:
    --topo-order        sort commits in topological order
    --date-order        sort commits in date order
    --reverse           sort commits in reverse
    -h, --help          show this message
";

    let args = Docopt::new(USAGE).and_then(|d| d.decode())
                                 .unwrap_or_else(|e| e.exit());
    match run(&args) {
        Ok(()) => {}
        Err(e) => println!("error: {}", e),
    }
}
