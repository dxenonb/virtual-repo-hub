use virtual_repo_hub::get_status;
use virtual_repo_hub::config::{
    Config,
    ConfigError,
    config_path,
};

use git2::Repository;

use std::env;

fn main() -> Result<(), i32> {
    let config_path = config_path().unwrap();
    println!("Using {} as config directory!", config_path.to_str().unwrap());

    println!("Attempting to load config...");
    let config = match Config::load(&config_path) {
        Ok(config) => config,
        Err(ConfigError::NotFound) => Config::init(&config_path)
            .expect("Failed to init config")
            .unwrap(),
        Err(err) => panic!("Failed to find or init config: {:?}", err),
    };
    println!("Loaded config: {:?}", &config);

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
