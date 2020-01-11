use crate::RepoStatus;

use serde::{Serialize, Deserialize};
use tempfile::{tempdir, TempDir};

use std::collections::HashMap;
use std::process::{Command, ExitStatus};
use std::env::{current_dir, set_current_dir};
use std::fs;
use std::io::{Write, BufWriter};

struct GenState {
    repos: HashMap<String, TempDir>,
    active: Option<String>,
}

impl GenState {
    fn new() -> Self {
        GenState {
            repos: HashMap::new(),
            active: None,
        }
    }

    fn init(&mut self, bare: bool) {
        if let Some(_) = self.active {
            unimplemented!();
        }

        let temp_dir = tempdir().unwrap();

        set_current_dir(temp_dir.path())
            .expect("failed to set working directory for new repo");

        let args = if bare { &["init"][..] } else { &["init", "--bare"][..] };
        assert!(run_git(args).success());

        let name = "origin";
        self.repos.insert(name.to_string(), temp_dir);
        self.active = Some(name.to_string());
    }

    fn commit(&mut self, repeat: u32) {
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

        for _ in 0..repeat {
            f.write_all(b"\tcommit!")
                .unwrap();
            f.flush()
                .unwrap();

            let status = run_git(&["commit", "-m", "arbitrary commit", "--", file_name]);
            assert!(status.success());
        }
    }

    fn assert_active(&self) {
        assert!(self.active.is_some(), "no git repo is active");
        let td = self.repos.get(self.active.as_ref().unwrap())
            .unwrap()
            .path();
        assert_eq!(current_dir().unwrap(), td);
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all="snake_case")]
enum GenCommand {
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
    fn execute(&self, state: &mut GenState) {
        use GenCommand::*;
        match self {
            Init { bare } => state.init(*bare),
            Commit { repeat } => state.commit(*repeat),
            _ => unimplemented!(),
        }
    }
}

fn run_git(args: &[&str]) -> ExitStatus {
    Command::new("git")
        .args(args)
        .status()
        .expect("failed to run git command")
}

fn r#false() -> bool {
    false
}

mod test {
    use super::*;

    const EXAMPLE1: &str = "
        - init: {}
        - commit:
            repeat: 3
        - modify: {}
        - stage: {}
        - expect:
            status:
                bare: false
                clean_status: true
                clean_state: true
                stashes: 0
                remotes: []
                branches: {}";

    #[test]
    #[should_panic]
    fn detects_inactive_state() {
        let state = GenState::new();
        state.assert_active();
    }

    #[test]
    #[should_panic]
    fn detects_evaded_state() {
        use std::env;

        let mut state = GenState::new();
        state.init(false);

        set_current_dir(env::var("HOME").unwrap())
            .unwrap();

        state.assert_active();
    }

    #[test]
    fn asserts_active() {
        let mut state = GenState::new();
        state.init(false);
        state.assert_active();
    }

    #[test]
    fn parses_commands() {
        let commands: Vec<GenCommand> = serde_yaml::from_str(EXAMPLE1)
            .unwrap();

        let status = match &commands.last().unwrap() {
            GenCommand::Expect { status } => status,
            _ => panic!(),
        };

        assert_eq!(status, &RepoStatus {
            bare: false,
            clean_status: true,
            clean_state: true,
            stashes: 0,
            remotes: Vec::new(),
            branches: HashMap::new(),
        });
    }
}
