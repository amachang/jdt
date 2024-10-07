use std::{fs, path::{Path, PathBuf}, io};
use dirs::config_dir;
use serde::{Deserialize, Serialize};

pub struct Project {
    pub project_name: String,
    pub config_dir: PathBuf,
}

impl Project {
    pub fn new(project_name: impl Into<String>) -> Self {
        let project_name = project_name.into();
        let config_parent_dir = config_dir().expect("Failed to get config directory");
        let config_dir = config_parent_dir.join(&project_name);
        Self {
            project_name,
            config_dir,
        }
    }

    pub fn config<Config: Deserialize<'static> + Serialize + Default>(&self) -> Config {
        let config_path = self.config_dir.join("config.toml");
        fs::create_dir_all(&self.config_dir).expect("Failed to create config directory");

        if !config_path.exists() {
            let default_config = Config::default();
            let toml = match toml::to_string_pretty(&default_config) {
                Ok(toml) => toml,
                Err(err) => panic!("Failed to serialize default config: {}", err),
            };
            match fs::write(&config_path, toml) {
                Ok(_) => (),
                Err(err) => panic!("Failed to write default config: {}", err),
            }
            log::debug!("Default config written to {}", config_path.display());
        }

        let config = match config::Config::builder().add_source(config::File::from(config_path.as_path())).build() {
            Ok(config) => config,
            Err(err) => panic!("Failed to build config: {}", err),
        };
        let config = match config.try_deserialize::<Config>() {
            Ok(config) => config,
            Err(err) => panic!("Failed to deserialize config: {}", err),
        };

        config
    }
}

pub fn walk_dir<R>(dir: impl AsRef<Path>, mut f: impl FnMut(PathBuf) -> R) -> Vec<R> {
    // must be implement no recursive
    let mut dir_stack = vec![dir.as_ref().to_path_buf()];
    let mut results = Vec::new();
    while let Some(dir) = dir_stack.pop() {
        let iter = match fs::read_dir(&dir) {
            Ok(iter) => iter,
            Err(err) => {
                log::warn!("Ignoring error {} in {}", err, dir.display());
                continue;
            }
        };
        for entry in iter {
            let entry = match entry {
                Ok(entry) => entry,
                Err(err) => {
                    log::warn!("Ignoring error {} in {}", err, dir.display());
                    continue;
                }
            };
            let path = entry.path();
            if path.is_dir() {
                dir_stack.push(path);
            } else {
                results.push(f(path));
            }
        }
    }
    results
}

pub fn project(project_name: impl Into<String>) -> Project {
    Project::new(project_name)
}

pub fn almost_eq<F: num_traits::Float>(a: F, b: F, relative_tolerance: F) -> bool {
    let min = a.min(b);
    let max = a.max(b);
    ((max - min) / max) <= relative_tolerance
}

// allow to rename file across different filesystems
pub fn rename_file(from_path: impl AsRef<Path>, to_path: impl AsRef<Path>) -> Result<(), io::Error> {
    let from_path = from_path.as_ref();
    let to_path = to_path.as_ref();
    match fs::rename(from_path, to_path) {
        Ok(_) => Ok(()),
        Err(e) => {
            match e.raw_os_error() {
                Some(libc::EXDEV) => {
                    fs::copy(from_path, to_path)?;
                    fs::remove_file(from_path)?;
                    Ok(())
                },
                _ => Err(e),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_walk_dir() {
        let files = walk_dir("./src", |path| path);
        assert!(files.len() > 0);
        assert!(files.iter().find(|path| path.ends_with("lib.rs")).is_some());
    }

    #[test]
    fn test_almost_eq() {
        assert!(almost_eq(1.0, 1.0, 0.0));
        assert!(almost_eq(1.0, 1.0, 0.1));
        assert!(almost_eq(1.0, 1.1, 0.1));
        assert!(!almost_eq(1.0, 1.1, 0.01));
    }
}

