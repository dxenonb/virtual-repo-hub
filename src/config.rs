use uuid::Uuid;
use serde::{Serialize, Deserialize};

use std::path::{
    PathBuf,
    Path,
};
use std::collections::HashMap;
use std::fs;
use std::io;
use std::io::{
    BufReader,
    BufWriter,
};
use std::env;

// TODO: Version handling. Current thought is to create a `version` file, but
// only check if it serializing/deserializing fails... seems annoying to read
// *yet another* different file every time the app is run...

const DEVICE_ID_PATH: &str = "deviceid";
const HUB_ID_PATH: &str = "hub";
const DEVICE_CONFIG_DIR: &str = "device";
const HUB_CONFIG_DIR: &str = "hubs";

const DEFAULT_HUB: &str = "default";

#[derive(Debug)]
pub struct Config {
    // hub: HubConfig,
    device: Device,
}

impl Config {
    pub fn load<P: AsRef<Path>>(path: P) -> Result<Config, ConfigError> {
        let mut path = PathBuf::from(path.as_ref());

        path.push(DEVICE_ID_PATH);
        let id = fs::read_to_string(&path)?;

        path.pop();
        path.push(HUB_ID_PATH);
        let hub = fs::read_to_string(&path)?;

        path.pop();
        Config::device_config_path(&mut path, &hub);
        let reader = BufReader::new(fs::File::open(&path)?);
        let device_config = serde_json::from_reader(reader)?;
        let device = Device {
            id,
            hub,
            config: device_config,
        };

        Ok(Config {
            device,
        })
    }

    /// Initialize configuration for this device
    ///
    /// Returns Ok(None) if the device appears to already be initialized.
    pub fn init<P: AsRef<Path>>(path: P) -> Result<Option<Config>, ConfigError> {
        let mut path = PathBuf::from(path.as_ref());
        path.push(DEVICE_ID_PATH);
        if path.exists() {
            // TODO: do a more fine grained check on what the problem is
            return Ok(None);
        }

        let device = Device::new();

        // write the device id
        fs::write(&path, &device.id)?;
        // write the hub id
        path.pop();
        path.push(HUB_ID_PATH);
        fs::write(&path, &device.hub)?;

        // write the device config
        path.pop();
        Config::device_config_path(&mut path, &device.hub);
        fs::create_dir_all(path.parent().unwrap())?;
        let writer = BufWriter::new(fs::File::create(&path)?);
        serde_json::to_writer_pretty(writer, &device.config)?;

        Ok(Some(Config {
            device,
        }))
    }

    fn device_config_path(config_path: &mut PathBuf, hub: &str) {
        config_path.push(DEVICE_CONFIG_DIR);
        config_path.push(&hub);
        config_path.set_extension("json");
    }
}

#[derive(Debug)]
struct Device {
    id: String,
    hub: String,
    config: DeviceConfig,
}

impl Device {
    fn new() -> Device {
        let id = Uuid::new_v4().to_string();
        let hub = DEFAULT_HUB.to_string();
        let config = DeviceConfig {
            starred: HashMap::new(),
        };
        Device {
            id,
            hub,
            config,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct DeviceConfig {
    /// Directories that will be indexed.
    starred: HashMap<String, StoredPath>,
    // /// Directories that are meant to always describe an env.
    // env_dir: HashMap<String, StoredPath>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(transparent)]
struct StoredPath(String);

#[derive(Debug)]
pub enum ConfigError {
    NotFound,
    Io(io::Error),
    Parsing(serde_json::Error),
}

impl From<io::Error> for ConfigError {
    fn from(err: io::Error) -> ConfigError {
        if let io::ErrorKind::NotFound = err.kind() {
            ConfigError::NotFound
        } else {
            ConfigError::Io(err)
        }
    }
}

impl From<serde_json::Error> for ConfigError {
    fn from(err: serde_json::Error) -> ConfigError {
        ConfigError::Parsing(err)
    }
}

pub fn config_path() -> Result<PathBuf, &'static str> {
    const ERR: &str = "Neither HOME or VIRTUAL_REPO_HUB_HOME was defined";

    Ok(match path_var("VIRTUAL_REPO_HUB_HOME") {
        Some(path) => path,
        None => match path_var("HOME") {
            Some(mut path) => {
                // don't take responsibility for creating .config if it doesn't exist
                path.push(".config");
                if path.exists() {
                    path.push("virtual-repo-hub");
                } else {
                    path.pop();
                    path.push(".virtual-repo-hub");
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

// struct HubConfig {
//     /// Network sources that can provide many repos and information about them.
//     providers: HashMap<String, ProviderRef>,
//     /// Repos that are referenced directly by their URLs.
//     repos: HashMap<String, GitUrl>,
//     /// Git config profiles for quickly setting and swapping configs
//     profiles: HashMap<String, GitProfile>,
//     /// Virtual env definitions
//     envs: HashMap<String, EnvDef>,
//     /// Envs that are starred as "top level" directories.
//     ///
//     /// They may still be nested inside other envs.
//     roots: Vec<String>,
// }

// enum ProviderRef {
//     GitHub,
//     GitLab,
//     Bitbucket,
// }

// struct GitUrl(String);

// struct GitProfile {
//     name: String,
//     email: String,
// }

// struct EnvDef {
//     /// Starred directories pinned inside this env.
//     ///
//     /// If the directory does not exist on this device it is ignored.
//     pinned: Vec<String>,
//     providers: Vec<String>,
//     env: Vec<String>,
// }

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
