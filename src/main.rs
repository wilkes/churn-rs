// churn - Count how many versions exist of each file in a git repository.

extern crate git2;
extern crate docopt;

use docopt::Docopt;
use git2::{Repository, Error, Oid, Tree, ObjectType};
use std::collections::{HashMap, HashSet};
use std::io::Write;

/// Get or create a HashMap entry.
///
/// If the given `map` does *not* already have an entry with the given `key`,
/// this inserts the pair `(key, f())` into the map.
///
/// Returns a mut reference to `map[key]`.
///
fn get_mut_or_create_with<'a, V, F: FnOnce()->V>(
    map: &'a mut HashMap<String, V>, key: &str, f: F) -> &'a mut V
{
    // Pure optimization: the one-liner below is correct, but since this path
    // is hot, we indulge in a little unsafe code to avoid the expense of
    // `key.to_string()` when the entry already exists (the common case for
    // most repositories).
    unsafe {
        let p_map: *mut HashMap<String, V> = map;
        if let Some(r) = (*p_map).get_mut(key) {
            return r;
        }
    }

    map.entry(key.to_string()).or_insert_with(f)
}

/// Join a directory path `base` to a filename `name`.
fn join(base: &str, name: &str) -> String {
    match base {
        "" => name.to_string(),
        _ => base.to_string() + "/" + name
    }
}

/// Cumulative version counts for everything under one directory of a
/// repository, including subdirectories.
///
/// The basic algorithm here is to create a root `DirData`, update it for every
/// commit in the repository, then get the results out of the resulting tree of
/// `DirData` records.
struct DirData {
    /// Set of all Git "tree" hashes ever seen for this directory.
    ///
    /// A Git "tree" is a snapshot of a directory. When we query Git for a
    /// given commit, Git doesn't give us a patch telling what was changed in
    /// that commit. Instead, it gives us a complete snapshot of *all* files
    /// and directories in that commit, including files and directories that
    /// did not change.
    ///
    /// Therefore we have to keep the set of all hashes we've seen for every
    /// directory (this field) and every file (`DirData::files`) to avoid
    /// overcounting directories or doing redundant work.
    hashes: HashSet<Oid>,

    /// Table of all blob hashes ever seen for each file in this directory.
    files: HashMap<String, HashSet<Oid>>,

    /// Each subdirectory that ever existed under this directory gets its own
    /// `DirData` record.
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

    /// Get a `DirData` record for a subdirectory of this dir, creating a new
    /// record if we don't already have one.
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

const COMMITS_PER_DOT: usize = 1000;

fn run(dirname: &str) -> Result<(), git2::Error> {
    let repo = try!(Repository::open(dirname));
    let mut revwalk = try!(repo.revwalk());
    revwalk.set_sorting(git2::SORT_NONE);
    let spec = "HEAD";

    let mut root_dir: DirData = DirData::new();

    let id:Oid = try!(repo.revparse_single(spec)).id();
    try!(revwalk.push(id));
    let mut n = 0;
    for id in revwalk {
        let commit = try!(repo.find_commit(try!(id)));
        let tree = try!(commit.tree());
        try!(root_dir.update_for_tree(&repo, &tree));

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
usage: gitlog [options] [<dir>]

Options:
    -h, --help          show this message
";

    let args =
        Docopt::new(USAGE)
        .and_then(|d| d.parse())
        .unwrap_or_else(|e| e.exit());
    let dir = match args.get_str("<dir>") {
        "" => ".",
        d => d
    };
    match run(dir) {
        Ok(()) => {}
        Err(e) => println!("error: {}", e),
    }
}
