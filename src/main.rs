use virtual_repo_hub::get_status;
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

const STATUS_ABOUT: &str = "Get advanced status information from one or more repos quickly.";
const STATUS_HELP: &str = "Get advanced status information from one or more repos quickly."; // TODO

fn main() -> Result<(), i32> {
    let config_path = config_path().unwrap();
    let config = match Config::load(&config_path) {
        Ok(config) => config,
        Err(ConfigError::NotFound) => Config::init(&config_path)
            .expect("Failed to init config")
            .unwrap(),
        Err(err) => panic!("Failed to find or init config: {:?}", err),
    };

    let app = App::new("Virtual Repo Hub")
        .version("0.1")
        .about("Tools for managing many repositories in many places.")
        .setting(AppSettings::ArgRequiredElseHelp)
        .subcommand(SubCommand::with_name("star")
            .about("Add a directory to your starred directories.")
            .arg(Arg::with_name("DIR")
                .required(true)
                .help("directory to star")))
        .subcommand(SubCommand::with_name("status")
            .about(STATUS_ABOUT)
            .help(STATUS_HELP)
            .arg(Arg::with_name("DIR")
                .required(true)));

    let matches = app.get_matches();

    match matches.subcommand() {
        ("star", Some(_matches)) => {
            unimplemented!();
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
        (_, _) => unreachable!(),
    }

    Ok(())
}
