use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Result, anyhow};

use crate::io::{read_toml, write_toml};
use crate::types::Config;

pub(crate) fn bmx_home() -> Result<PathBuf> {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .ok_or_else(|| anyhow!("cannot determine home dir from HOME"))?;
    Ok(home.join(".bmx"))
}

pub(crate) fn ensure_base_dirs(home: &Path) -> Result<()> {
    fs::create_dir_all(home.join("apps"))?;
    if !home.join("config.toml").exists() {
        save_config(home, &Config::default())?;
    }
    Ok(())
}

pub(crate) fn load_config(home: &Path) -> Result<Config> {
    let cfg_file = home.join("config.toml");
    if !cfg_file.exists() {
        return Ok(Config::default());
    }
    read_toml(&cfg_file)
}

pub(crate) fn save_config(home: &Path, config: &Config) -> Result<()> {
    write_toml(&home.join("config.toml"), config)
}
