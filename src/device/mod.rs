mod format;
mod fraction;
mod info;
mod settings;
mod size;

use confique::Config;
pub use format::*;
pub use fraction::*;
pub use info::*;
use serde::Serialize;
pub use settings::*;
pub use size::*;

#[derive(Serialize, Config, Debug, Clone)]
pub struct JustDevice {
    #[config(nested)]
    pub info: DeviceInfo,
    #[config(nested)]
    pub settings: DeviceSettings,
}
