use git2::{Repository, RepositoryState};

#[cfg(test)]
use serde::{Serialize, Deserialize};

use std::env;
use std::collections::HashMap;

#[cfg(test)]
mod test;

#[derive(Debug, PartialEq)]
#[cfg_attr(test,
    derive(Serialize, Deserialize),
    serde(rename_all="snake_case"))
]
pub struct RepoStatus {
    pub bare: bool,
    /// True if nothing is staged and there are no untracked files.
    pub clean_status: bool,
    /// True if there is no conflict resolution in progress.
    pub clean_state: bool,
    pub stashes: usize,
    pub remotes: Vec<Remote>,
    pub branches: HashMap<String, BranchStatus>,
}

#[derive(Debug, PartialEq)]
#[cfg_attr(test,
    derive(Serialize, Deserialize),
    serde(rename_all="snake_case"))
]
pub struct Remote {
    name: String,
}

#[derive(Debug, PartialEq)]
#[cfg_attr(test,
    derive(Serialize, Deserialize),
    serde(rename_all="snake_case"))
]
pub enum BranchStatus {
    TrackingBranch(TrackingStatus),
    LocalBranch {
        merged_in_remote: bool,
    },
}

impl BranchStatus {
    fn new_tracking_branch(tracking_status: TrackingStatus) -> Self {
        BranchStatus::TrackingBranch(tracking_status)
    }

    fn new_local_branch(merged_in_remote: bool) -> Self {
        BranchStatus::LocalBranch { merged_in_remote }
    }


    fn merged_in_upstream(&self) -> bool {
        let tracking_status = match self {
            BranchStatus::TrackingBranch(status) => status,
            BranchStatus::LocalBranch { merged_in_remote } => return *merged_in_remote,
        };

        use TrackingStatus::*;
        match tracking_status {
            Diverged | Ahead => false,
            Behind | Current => true,
        }
    }
}

#[derive(Debug, PartialEq)]
#[cfg_attr(test,
    derive(Serialize, Deserialize),
    serde(rename_all="snake_case"))
]
pub enum TrackingStatus {
    Diverged,
    Ahead,
    Behind,
    Current,
}

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

    // TODO: figure out if there should be an early return at all
    if bare {
        return Ok(RepoStatus {
            bare,
            remotes,
            clean_status: true,
            clean_state: true,
            stashes: 0,
            branches: HashMap::new(),
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
    repo.stash_foreach(|_, _, _| {
        stashes += 1;
        true
    })?;

    let mut local_only_branches = Vec::new();
    let mut branches = HashMap::new();

    let mut branch_iter = repo.branches(Some(git2::BranchType::Local))?;
    for branch in branch_iter {
        let (branch, _) = branch?;
        let tracking_status = match branch.upstream() {
            Ok(upstream) => {
                // get the merge base
                let upstream_commit = upstream.get().peel_to_commit()?.id();
                let branch_commit = branch.get().peel_to_commit()?.id();

                if upstream_commit == branch_commit {
                    TrackingStatus::Current
                } else {
                    // TODO: The error codes aren't document for this...
                    let ancestor = repo.merge_base(branch_commit, upstream_commit)
                        .expect("merge base error");
                    if ancestor == branch_commit {
                        // if the merge base is branch, then branch is behind the upstream
                        TrackingStatus::Behind
                    } else if ancestor == upstream_commit {
                        // if the merge base is the upstream, then branch is ahead
                        TrackingStatus::Ahead
                    } else {
                        // if it is neither, the branches have diverged
                        TrackingStatus::Diverged
                    }
                }
            },
            Err(err) => {
                if err.code() != git2::ErrorCode::NotFound {
                    return Err(err);
                }
                // add the branch to an auxillary list to be checked
                local_only_branches.push(branch);
                continue;
            }
        };

        let name = branch.name()?
            .unwrap_or_else(|| "[non utf-8]")
            .to_string();

        let status = BranchStatus::new_tracking_branch(tracking_status);
        branches.insert(name, status);
    }

    // loop over remote branches and check if the auxillary branches are merged
    branch_iter = repo.branches(Some(git2::BranchType::Local))?;
    for remote_branch in branch_iter {
        let (remote_branch, _) = remote_branch?;
        let remote_commit = remote_branch.get().peel_to_commit()?.id();
        let mut iter_err = None;
        local_only_branches.retain(|branch| {
            let commit = match branch.get().peel_to_commit() {
                Ok(commit) => commit.id(),
                Err(err) => {
                    iter_err = Some(err);
                    return false;
                },
            };
            let ancestor = repo.merge_base(commit, remote_commit)
                .expect("merge base error");

            let result = ancestor == commit;
            if result {
                let name = match branch.name() {
                    Ok(branch) => branch.unwrap_or_else(|| "[non utf-8]")
                        .to_string(),
                    Err(err) => {
                        iter_err = Some(err);
                        return false;
                    },
                };
                branches.insert(name, BranchStatus::new_local_branch(true));
            }

            result
        });

        if let Some(err) = iter_err {
            return Err(err);
        }

        if local_only_branches.is_empty() {
            break;
        }
    }

    for local_branch in local_only_branches {
        let name = local_branch.name()?
            .unwrap_or_else(|| "[non utf-8]")
            .to_string();
        branches.insert(name, BranchStatus::new_local_branch(false));
    }

    Ok(RepoStatus {
        bare: false,
        clean_status,
        clean_state,
        stashes,
        remotes,
        branches,
    })
}
