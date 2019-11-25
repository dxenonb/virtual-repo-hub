use git2::{Repository, RepositoryState};

use std::env;

#[derive(Debug)]
struct RepoStatus {
    bare: bool,
    /// True if nothing is staged and there are no untracked files.
    clean_status: bool,
    /// True if there is no conflict resolution in progress.
    clean_state: bool,
    stashes: usize,
    remotes: Vec<Remote>,
    // branches: HashMap<String, BranchStatus>,
}

#[derive(Debug)]
struct Remote {
    name: String,
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

    let mut repo = match Repository::open(path.clone()) {
        Ok(repo) => repo,
        Err(_) => {
            eprintln!("Failed to open a git repo at {}", &path);
            return Err(-1);
        }
    };

    let status = get_status(&mut repo)
        .expect("Failed to get repo status");

    println!("Got repo status: {:?}", &status);

    Ok(())
}

fn get_status(repo: &mut Repository) -> Result<RepoStatus, git2::Error> {
    let bare = repo.is_bare();

    let remotes = {
        let mut out = Vec::new();
        let remotes = repo.remotes()?;
        for remote in remotes.iter() {
            let name = remote.map(|s| s.to_string())
                .unwrap_or_else(|| "[non unicode]".to_string());
            out.push(Remote { name })
        }
        out
    };

    if bare {
        return Ok(RepoStatus {
            bare,
            remotes,
            clean_status: true,
            clean_state: true,
            stashes: 0,
        });
    }

    let mut clean_status = true;
    {
        let statuses = repo.statuses(None)?;

        let cmp_status = git2::Status::CURRENT | git2::Status::IGNORED;
        for entry in statuses.iter() {
            if !entry.status().intersects(cmp_status) {
                clean_status = false;
                break;
            }
        }
    }

    let clean_state = repo.state() == RepositoryState::Clean;

    let mut stashes = 0;
    repo.stash_foreach(|i, s, _| {
        println!("Got stash: {}, {}", i, s);
        stashes += 1;
        true
    })?;

    Ok(RepoStatus {
        bare: false,
        clean_status,
        clean_state,
        stashes,
        remotes,
    })
}
