use std::{fs, path::{Path, PathBuf}, sync::Mutex};
use dirs::config_dir;
use serde::{Deserialize, Serialize};
use once_cell::sync::Lazy;

static PROJECT_NAME: Lazy<Mutex<Option<String>>> = Lazy::new(|| Mutex::new(None));
static CONFIG_PARENT_DIR: Lazy<PathBuf> = Lazy::new(|| config_dir().expect("Config directory not found"));

pub fn use_from(project_name: impl AsRef<str>) {
    let project_name = project_name.as_ref();
    let mut project_name_var = PROJECT_NAME.lock().expect("Failed to initialize jdt library by acquiring lock");
    *project_name_var = Some(project_name.to_string());
}

pub fn config<Config: Deserialize<'static> + Serialize + Default>() -> Config {
    let project_name = {
        let project_name = PROJECT_NAME.lock().expect("Failed to initialize jdt library by acquiring lock");
        project_name.clone().expect("Project name not set")
    };
    let config_dir = CONFIG_PARENT_DIR.join(project_name);
    let config_path = config_dir.join("config.toml");
    match fs::create_dir_all(config_dir) {
        Ok(_) => (),
        Err(err) => panic!("Failed to create config directory: {}", err),
    }

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

pub fn almost_eq<F: num_traits::Float>(a: F, b: F, relative_tolerance: F) -> bool {
    let min = a.min(b);
    let max = a.max(b);
    ((max - min) / max) <= relative_tolerance
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

