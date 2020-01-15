use virtual_repo_hub::get_status;

use git2::Repository;

use std::env;
use std::path::PathBuf;

fn main() -> Result<(), i32> {
    println!("Using {} as config directory!", config_path().unwrap().to_str().unwrap());
    
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

fn config_path() -> Result<PathBuf, &'static str> {
    const ERR: &str = "Neither HOME or VIRTUAL_REPO_HUB_HOME was defined";

    Ok(match path_var("VIRTUAL_REPO_HUB_HOME") {
        Some(path) => path,
        None => match path_var("HOME") {
            Some(mut path) => {
                // don't take responsibility for creating .config if it doesn't exist
                path.push(".config");
                if path.exists() {
                    path.push("virtual_repo_hub");
                } else {
                    path.pop();
                    path.push(".virtual_repo_hub");
                }
                path
            },
            None => return Err(ERR),
        },
    })
}

fn path_var<'a, 'b>(key: &'a str) -> Option<PathBuf> {
    use env::VarError as E;

    Some(match env::var(key) {
        Ok(home) => PathBuf::from(home),
        Err(E::NotUnicode(home)) => PathBuf::from(home),
        Err(E::NotPresent) => return None,
    })
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn gets_config_path() {
        // .config is used when it exists
        env::set_var("HOME", "tests");

        let mut p = config_path().unwrap();
        assert_eq!(PathBuf::from("tests/.config/virtual_repo_hub"), p);

        // .virtual_repo_hub is used when .config doesn't exist
        env::set_var("HOME", "fakehome");

        p = config_path().unwrap();
        assert_eq!(PathBuf::from("fakehome/.virtual_repo_hub"), p);

        // env var overrides everything when used
        env::set_var("VIRTUAL_REPO_HUB_HOME", "foo/bar/baz");

        p = config_path().unwrap();
        assert_eq!(PathBuf::from("foo/bar/baz"), p);
    }
}
