use git2::{Repository, RepositoryState};

use std::collections::HashMap;

// struct RemoteUrl(String);

#[derive(Debug)]
struct RepoStatus {
    clean_tree: bool,
    clean_index: bool,
    clean_state: bool,
    // remotes: Vec<RemoteUrl>,
    // stashes: usize,
    // branches: HashMap<String, BranchStatus>,
}

/*
struct BranchStatus {
    tracking_status: TrackingStatus,
    merged_in_tracking: bool,
}

enum TrackingStatus {
    Diverged,
    Ahead,
    Behind,
}
*/

fn main() {
    println!("Hello, world!");

    let repo = Repository::open("../jaredrenzi-website")
        .expect("Failed to open a git repo at ./");

    let status = get_status(&repo)
        .expect("Failed to get repo status");

    println!("Got repo status: {:?}", &status);
}

fn get_status(repo: &Repository) -> Result<RepoStatus, git2::Error> {
    let headref = repo.head()?;
    let tree = headref.peel_to_tree()?;
    let diffs = repo.diff_tree_to_workdir(Some(&tree), None)?;
    let clean_tree = diffs.deltas().next().is_none();
    let index = repo.index()?;
    for i in index.iter() {
        println!("Index: {:?}", std::str::from_utf8(&i.path).unwrap());
    }
    let clean_index = repo.index()?.is_empty();
    let clean_state = repo.state() == RepositoryState::Clean;

    Ok(RepoStatus {
        clean_tree,
        clean_index,
        clean_state,
    })
}
