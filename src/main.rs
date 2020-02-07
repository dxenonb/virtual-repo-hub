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

// TODO: Figure out the best way to get auto generated help with extra info.

const STATUS_ABOUT: &str = "Get advanced status information from one or more repos quickly.";
const STATUS_HELP: &str = "Get advanced status information from one or more repos quickly."; // TODO

const STAR_ABOUT: &str = "Add a directory to your starred directories.";
const STAR_HELP: &str = "Add a directory to your starred directories.

Starred directories are directories that largely contain git repositories and/or that you would like
to track. VRH can use starred directory names as aliases, and when a directory is expected but not
provided, starred directories will often be used as defaults for VRH commands.";

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
        (_, _) => unreachable!(),
    }

    Ok(())
}
