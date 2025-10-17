use crate::device::{JustFraction, JustFrameSize};
use confique::Config;
use v4l::{FourCC, Fraction};

#[derive(Config, Debug, Clone)]
pub struct DeviceSettings {
    pub format: [u8; 4],
    #[config(nested)]
    pub size: JustFrameSize,

    #[config(nested)]
    pub fraction: JustFraction,
}

impl DeviceSettings {
    pub fn ffmpeg_r(&self) -> String {
        format!("{}/{}", self.fraction.denominator, self.fraction.numerator)
    }
}
