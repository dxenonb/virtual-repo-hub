use git2::{Repository, RepositoryState};

use std::env;

// struct RemoteUrl(String);

#[derive(Debug)]
struct RepoStatus {
    bare: bool,
    /// True if nothing is staged and there are no untracked files.
    clean_status: bool,
    /// True if there is no conflict resolution in progress.
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

fn main() -> Result<(), i32> {
    println!("Checking if repo is safely backed up...");

    let mut args = env::args().skip(1);
    let path = match args.next() {
        Some(path) => path,
        None => {
            eprintln!("Missing required path argument");
            return Err(-1);
        }
    };

    let repo = match Repository::open(path.clone()) {
        Ok(repo) => repo,
        Err(_) => {
            eprintln!("Failed to open a git repo at {}", &path);
            return Err(-1);
        }
    };

    let status = get_status(&repo)
        .expect("Failed to get repo status");

    println!("Got repo status: {:?}", &status);

    Ok(())
}

fn get_status(repo: &Repository) -> Result<RepoStatus, git2::Error> {
    let bare = repo.is_bare();

    if bare {
        return Ok(RepoStatus {
            bare,
            clean_status: true,
            clean_state: true,
        });
    }

    let statuses = repo.statuses(None)?;

    let cmp_status = git2::Status::CURRENT | git2::Status::IGNORED;
    let mut clean_status = true;
    for entry in statuses.iter() {
        if !entry.status().intersects(cmp_status) {
            clean_status = false;
            break;
        }
    }

    let clean_state = repo.state() == RepositoryState::Clean;

    Ok(RepoStatus {
        bare: false,
        clean_status,
        clean_state,
    })
}
