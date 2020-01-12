use virtual_repo_hub::RepoStatus;

use std::collections::HashMap;
use std::env::set_current_dir;

#[allow(dead_code)]
mod repo_gen;

use crate::repo_gen::*;

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
#[ignore]
fn detects_inactive_state() {
    let state = GenState::new(None);
    state.assert_active();
}

#[test]
#[should_panic]
fn detects_evaded_state() {
    use std::env;

    let mut state = GenState::new(None);
    state.init(false);

    set_current_dir(env::var("HOME").unwrap())
        .unwrap();

    state.assert_active();
}

#[test]
#[ignore]
fn asserts_active() {
    let mut state = GenState::new(None);
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
