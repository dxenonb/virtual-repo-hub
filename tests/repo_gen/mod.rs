use virtual_repo_hub::{RepoStatus, get_status_path};

use serde::{Serialize, Deserialize};
use tempfile::{tempdir, TempDir};

use std::collections::HashMap;
use std::process::{Command, Stdio};
use std::env::{current_dir, set_current_dir};
use std::fs;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::io;
use std::io::{Write, BufWriter};

const DEFAULT_FILE: &str = "default.txt";

pub struct GenState {
    use_directory: Option<(usize, PathBuf)>,
    repos: HashMap<String, RepoLocation>,
    active: Option<String>,
}

enum RepoLocation {
    Temp(TempDir),
    Perm(PathBuf),
}

impl RepoLocation {
    fn path(&self) -> &Path {
        match self {
            RepoLocation::Temp(dir) => dir.path(),
            RepoLocation::Perm(dir) => dir.as_path(),
        }
    }

    fn close(self) -> Result<(), io::Error> {
        match self {
            RepoLocation::Temp(dir) => dir.close(),
            _ => Ok(()),
        }
    }
}

impl GenState {
    // TODO: because we used canonicalize, we should no longer take a PathBuf
    pub fn new(use_directory: Option<PathBuf>) -> Self {
        GenState {
            use_directory: use_directory.map(|dir| {
                fs::create_dir_all(&dir)
                    .expect("failed to create full directory path");
                let dir = fs::canonicalize(dir)
                    .expect("failed to canonicalize use_directory");
                (1, dir)
            }),
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

        let temp_dir = self.alloc_dir().unwrap();

        set_current_dir(temp_dir.path())
            .expect("failed to set working directory for new repo");

        let args = if bare { &["init", "--bare"][..] } else { &["init"][..] };
        run_git(args);

        let name = "origin";
        self.repos.insert(name.to_string(), temp_dir);
        self.active = Some(name.to_string());

        self.config();
    }

    pub fn clone(&mut self) {
        self.assert_active();

        let clone = self.alloc_dir().unwrap();
        let clone_path = clone.path()
            .to_str()
            .unwrap();

        let active = self.active.as_ref()
            .unwrap();
        let source_path = self.repos.get(active)
            .unwrap()
            .path()
            .to_str()
            .unwrap();

        run_git(&["clone", source_path, clone_path]);

        set_current_dir(clone.path())
            .expect("failed to set working directory for cloned repo");

        let name = "clone";
        self.repos.insert(name.to_string(), clone);
        self.active = Some(name.to_string());

        self.config();
    }

    pub fn commit(&mut self, repeat: u32) {
        self.assert_active();

        if repeat == 0 {
            return;
        }

        let file_name = DEFAULT_FILE;
        let mut f = GenState::get_file(file_name);

        f.write_all(b"init\n")
            .unwrap();

        // we can't commit untracked files without notifying git they exist;
        // once the index is alerted to the new file though, "one stage" git-commit works fine
        run_git(&["add", file_name]);

        for _ in 0..repeat {
            f.write_all(b"\tcommit!\n")
                .unwrap();
            f.flush()
                .unwrap();

            run_git(&["commit", "-m", "arbitrary commit", file_name]);
        }
    }

    pub fn modify(&mut self) {
        let mut f = GenState::get_file(DEFAULT_FILE);
        f.write_all(b"modifying!\n")
            .unwrap();
        f.flush()
            .unwrap();
    }

    pub fn stage(&mut self) {
        run_git(&["add", DEFAULT_FILE]);
    }

    fn get_file<P: AsRef<Path>>(path: P) -> BufWriter<File> {
        let f = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .expect("failed to open file for appending");

        BufWriter::new(f)
    }

    fn alloc_dir(&mut self) -> Result<RepoLocation, io::Error> {
        match &mut self.use_directory {
            Some((i, dir)) => {
                let mut path = dir.clone();
                path.push(i.to_string());
                *i += 1;
                fs::create_dir_all(&path)
                    .expect("failed to create permanent dir");
                Ok(RepoLocation::Perm(path))
            },
            None =>
                tempdir()
                    .map(|dir| RepoLocation::Temp(dir))
        }
    }

    pub fn config(&self) {
        self.assert_active();

        run_git(&["config", "user.email", "foo.bar@example.com"]);
        run_git(&["config", "user.name", "Foo Bar"]);
    }

    /// Check that some kind of repo has been initialized and that the current working directory
    /// matches the appropriate location.
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
    Clone {},
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
            Clone {} => state.clone(),
            Commit { repeat } => state.commit(*repeat),
            Modify {} => state.modify(),
            Stage {} => state.stage(),
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
    execute_yaml_inner::<_, PathBuf>(path, None)
}

#[allow(dead_code)]
pub fn execute_yaml_in_dir<P1: AsRef<Path> + std::fmt::Debug, P2: Into<PathBuf>>(
    path: P1,
    dir: P2,
) -> Result<(), (usize, AssertionError)> {
    execute_yaml_inner(path, Some(dir))
}

fn execute_yaml_inner<P1: AsRef<Path> + std::fmt::Debug, P2: Into<PathBuf>>(
    path: P1,
    target_directory: Option<P2>,
) -> Result<(), (usize, AssertionError)> {
    let working_dir = current_dir()
        .unwrap();

    let contents = fs::read_to_string(&path)
        .expect(&format!("failed to open yaml at {:?}", &path));
    let commands: Vec<GenCommand> = serde_yaml::from_str(&contents)
        .expect(&format!("failed to read commands at {:?}", &path));

    let target_directory = target_directory.map(|dir| dir.into());
    let mut state = GenState::new(target_directory);
    let mut result = Ok(());
    for (i, cmd) in commands.iter().enumerate() {
        if let Err(err) = cmd.execute(&mut state) {
            result = Err((i, err));
            break;
        }
    }

    set_current_dir(&working_dir)
            .expect("failed to update working directory");

    state.cleanup();

    result
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
