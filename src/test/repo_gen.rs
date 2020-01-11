use crate::RepoStatus;

use serde::{Serialize, Deserialize};

use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all="snake_case")]
pub enum GenCommand {
    Init,
    Commit {
        repeat: u32,
    },
    Modify,
    Stage,
    Expect {
        status: RepoStatus,
    },
}

mod test {
    use super::*;

    const EXAMPLE1: &str = "
        - init
        - commit:
            repeat: 3
        - modify
        - stage
        - expect:
            status:
                bare: false
                clean_status: true
                clean_state: true
                stashes: 0
                remotes: []
                branches: {}";

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
