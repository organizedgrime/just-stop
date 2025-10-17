use anyhow::Result;
use confique::Config;
use etcetera::{BaseStrategy, choose_base_strategy};

use crate::device::JustDevice;

const CONFIG_FILE: &str = "config.kdl";

#[derive(Config)]
pub struct Conf {
    #[config(nested)]
    pub input: JustDevice,
    #[config(nested)]
    pub output: JustDevice,
}

impl Conf {
    pub fn load() -> Result<()> {
        let strategy = choose_base_strategy()?;
        strategy.config_dir();
        Ok(())
    }
}
