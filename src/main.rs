use std::ffi::OsStr;

use virtual_repo_hub::{get_status, BranchStatus, TrackingStatus};
use virtual_repo_hub::config::{
    Config,
    ConfigError,
    config_path,
};

use git2::Repository;
use clap::{
    App,
    AppSettings,
    Arg,
    SubCommand,
};

// TODO: Figure out the best way to get auto generated help with extra info.

const STATUS_ABOUT: &str = "Get advanced status information from one or more repos quickly.";
const STATUS_HELP: &str = "Get advanced status information from one or more repos quickly."; // TODO

const STAR_ABOUT: &str = "Add a directory to your starred directories.";
const STAR_HELP: &str = "Add a directory to your starred directories.

Starred directories are directories that largely contain git repositories and/or that you would like
to track. VRH can use starred directory names as aliases, and when a directory is expected but not
provided, starred directories will often be used as defaults for VRH commands.";

const BACKUPCHECK_ABOUT: &str = "Check if a directory or git repo is fully backed up.";
const BACKUPCHECK_HELP: &str = "Check if a directory or git repo is fully backed up.";

fn main() -> Result<(), i32> {
    let config_path = config_path().unwrap();
    let mut config = match Config::load(&config_path) {
        Ok(config) => config,
        Err(ConfigError::NotFound) => Config::init(&config_path)
            .expect("Failed to init config")
            .unwrap(),
        Err(err) => panic!("Failed to find or init config: {:?}", err),
    };

    let app = App::new("Virtual Repo Hub")
        .version("0.1")
        .set_term_width(80)
        .about("Tools for managing many repositories in many places.")
        .setting(AppSettings::ArgRequiredElseHelp)
        .subcommand(SubCommand::with_name("star")
            .about(STAR_ABOUT)
            .help(STAR_HELP)
            .arg(Arg::with_name("DIR")
                .required(true)
                .help("directory to star"))
            .arg(Arg::with_name("ALIAS")
                .help("alias to represent the starred directory")))
        .subcommand(SubCommand::with_name("status")
            .about(STATUS_ABOUT)
            .help(STATUS_HELP)
            .arg(Arg::with_name("DIR")
                .required(true)))
        .subcommand(SubCommand::with_name("backupcheck")
            .about(BACKUPCHECK_ABOUT)
            .help(BACKUPCHECK_HELP)
            .arg(Arg::with_name("DIR")
                .required(true)));

    let matches = app.get_matches();

    match matches.subcommand() {
        ("star", Some(matches)) => {
            let dir = matches.value_of_os("DIR")
                .unwrap();
            let alias = matches.value_of_os("ALIAS");
            config.star(&dir, alias);
            config.save(&config_path)
                .expect("Failed to save configuration");
        },
        ("status", Some(matches)) => {
            let dir = matches.value_of_os("DIR")
                .unwrap();
            let mut repo = match Repository::open(dir.clone()) {
                Ok(repo) => repo,
                Err(_) => {
                    eprintln!("Failed to open a git repo at {:?}", &dir);
                    return Err(-1);
                }
            };

            let status = get_status(&mut repo)
                .expect("Failed to get repo status");

            println!("Got repo status: {:?}", &status);
        },
        ("backupcheck", Some(matches)) => {
            let dir = matches.value_of_os("DIR")
                .unwrap();

            if backup_check_dir(dir)? {
                println!("Determined repo to be clean: {:?}", dir);
            }
        },
        (_, _) => unreachable!(),
    }

    Ok(())
}

fn backup_check_dir(dir: &OsStr) -> Result<bool, i32> {
    let mut repo = match Repository::open(dir.clone()) {
        Ok(repo) => repo,
        Err(_) => {
            eprintln!("Failed to open a git repo at {:?}", &dir);
            return Err(-1);
        }
    };

    let status = get_status(&mut repo)
        .expect("Failed to get repo status");

    if status.bare {
        println!("Did not check bare repo");
        return Ok(false);
    }

    if !status.clean_status {
        println!("Repo has modified/untracked files: {:?}", dir);
        return Ok(false);
    }

    if !status.clean_state {
        println!("Repo has a merge/rebase in progress: {:?}", dir);
        return Ok(false);
    }

    if status.remotes.len() == 0 {
        println!("Repo has no remotes! {:?}", dir);
        return Ok(false);
    }

    let mut clean = true;

    // check all the branches are up to date tracking branches or merged local branches
    for (_name, status) in &status.branches {
        match status {
            BranchStatus::LocalBranch { merged_in_remote: true } => {},
            BranchStatus::TrackingBranch(TrackingStatus::Behind | TrackingStatus::Current) => {},
            _ => {
                clean = false;
            },
        }
    }

    if !clean {
        println!("Repo has branches that are not backed up: {:?}", dir);
    }

    if status.stashes > 0 {
        if clean {
            println!("Repo {:?}:", dir);
        }
        println!("\tRepo has stashes: {}", status.stashes);
    }

    Ok(clean)
}
