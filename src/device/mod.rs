mod format;
mod info;
mod settings;
mod size;

pub use format::*;
pub use info::*;
pub use settings::*;
pub use size::*;

#[derive(Debug, Clone)]
pub struct JustDevice {
    pub info: DeviceInfo,
    pub settings: DeviceSettings,
}
