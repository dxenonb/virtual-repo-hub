use virtual_repo_hub::{RepoStatus, get_status_path};

use serde::{Serialize, Deserialize};
use tempfile::{tempdir, TempDir};

use std::collections::HashMap;
use std::process::{Command, ExitStatus, Stdio};
use std::env::{current_dir, set_current_dir};
use std::fs;
use std::path::Path;
use std::io::{Write, BufWriter};

pub struct GenState {
    repos: HashMap<String, TempDir>,
    active: Option<String>,
}

impl GenState {
    pub fn new() -> Self {
        GenState {
            repos: HashMap::new(),
            active: None,
        }
    }

    pub fn cleanup(self) {
        for (_, v) in self.repos {
            match v.close() {
                Err(err) => {
                    eprintln!("There was an issue cleaning up the generator state:");
                    eprintln!("\tTempDir reported an error deleting itself.");
                    eprintln!("{}", err);
                },
                Ok(_) => {},
            }
        }
    }

    pub fn init(&mut self, bare: bool) {
        if let Some(_) = self.active {
            unimplemented!();
        }

        let temp_dir = tempdir().unwrap();

        set_current_dir(temp_dir.path())
            .expect("failed to set working directory for new repo");

        let args = if bare { &["init", "--bare"][..] } else { &["init"][..] };
        run_git(args);

        let name = "origin";
        self.repos.insert(name.to_string(), temp_dir);
        self.active = Some(name.to_string());

        run_git(&["config", "user.email", "foo.bar@example.com"]);
        run_git(&["config", "user.name", "Foo Bar"]);
    }

    pub fn commit(&mut self, repeat: u32) {
        self.assert_active();

        if repeat == 0 {
            return;
        }

        let file_name = "default.txt";
        let f = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(file_name)
            .expect("failed to open file for committing");

        let mut f = BufWriter::new(f);

        f.write_all(b"init")
            .unwrap();

        // we can't commit untracked files without notifying git they exist;
        // once the index is alerted to the new file though, "one stage" git-commit works fine
        run_git(&["add", file_name]);

        for _ in 0..repeat {
            f.write_all(b"\tcommit!")
                .unwrap();
            f.flush()
                .unwrap();

            run_git(&["commit", "-m", "arbitrary commit", file_name]);
        }
    }

    pub fn assert_active(&self) {
        assert!(self.active.is_some(), "no git repo is active");
        let td = self.repos.get(self.active.as_ref().unwrap())
            .unwrap()
            .path();
        assert_eq!(current_dir().unwrap(), td);
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all="snake_case")]
pub enum GenCommand {
    Init {
        #[serde(default="r#false")]
        bare: bool,
    },
    Commit {
        repeat: u32,
    },
    Modify {},
    Stage {},
    Expect {
        status: RepoStatus,
    },
}

impl GenCommand {
    pub fn execute(&self, state: &mut GenState) -> Result<(), AssertionError> {
        use GenCommand::*;
        match self {
            Init { bare } => state.init(*bare),
            Commit { repeat } => state.commit(*repeat),
            Expect { status } => {
                let actual = get_status_path(current_dir().unwrap())
                    .expect("failed to get actual repo status");
                if status != &actual {
                    return Err(AssertionError {
                        expected: status.clone(),
                        actual,
                    });
                }
            },
            _ => unimplemented!(),
        }

        Ok(())
    }
}

pub struct AssertionError {
    pub expected: RepoStatus,
    pub actual: RepoStatus,
}

pub fn execute_yaml<P: AsRef<Path> + std::fmt::Debug>(
    path: P,
) -> Result<(), (usize, AssertionError)> {
    let contents = fs::read_to_string(&path)
        .expect(&format!("failed to open yaml at {:?}", &path));
    let commands: Vec<GenCommand> = serde_yaml::from_str(&contents)
        .expect(&format!("failed to read commands at {:?}", &path));

    let mut state = GenState::new();
    for (i, cmd) in commands.iter().enumerate() {
        if let Err(err) = cmd.execute(&mut state) {
            return Err((i, err));
        }
    }

    state.cleanup();

    Ok(())
}

fn run_git(args: &[&str]) {
    let status = Command::new("git")
        .args(args)
        .stdout(Stdio::null())
        .status()
        .expect("failed to run git command");
    assert!(status.success());
}

fn r#false() -> bool {
    false
}
