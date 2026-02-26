use std::{
    fs::{self, File},
    io::Write,
    path::PathBuf,
};

use anyhow::Result;
use confique::{toml::FormatOptions, Config};

use etcetera::{choose_base_strategy, BaseStrategy};
use serde::Serialize;

mod device;
pub mod effects;
mod files;

pub mod stream;
pub use device::*;
pub use files::*;

use effects::JustEffects;
use stream::JustStream;

const CONFIG_FILE: &str = "config.toml";

#[derive(Serialize, Config)]
pub struct Conf {
    #[config(nested)]
    pub input: JustDevice,

    #[config(nested)]
    pub output: JustStream,

    #[config(nested)]
    pub effects: JustEffects,
}

impl Conf {
    pub fn get_dir() -> Result<PathBuf> {
        let strategy = choose_base_strategy()?;
        let folder = strategy.config_dir().join("just-stop");
        Ok(folder)
    }

    pub fn get_path() -> Result<PathBuf> {
        Ok(Self::get_dir()?.join(CONFIG_FILE))
    }

    pub fn load() -> Result<Conf> {
        let conf = Conf::from_file(Conf::get_path()?)?;
        Ok(conf)
    }

    pub fn setup() -> Result<()> {
        let dir = Self::get_dir()?;
        let path = Self::get_path()?;

        if !dir.exists() {
            fs::create_dir(dir)?;
        }

        if !path.exists() {
            println!("Creating new template file for configuration...");
            let mut file = File::create(Conf::get_path()?)?;
            let toml = confique::toml::template::<Conf>(FormatOptions::default());
            file.write_all(toml.as_bytes())?;
        }

        Ok(())
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::get_path()?;
        let toml_string = toml::to_string_pretty(self)?;
        fs::write(path, toml_string)?;
        Ok(())
    }
}
