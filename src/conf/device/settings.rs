use crate::conf::device::{JustFraction, JustFrameSize};
use confique::Config;
use serde::Serialize;
use v4l::{FourCC, Fraction};

#[derive(Serialize, Config, Debug, Clone)]
pub struct DeviceSettings {
    pub format: [u8; 4],
    #[config(nested)]
    pub size: JustFrameSize,

    #[config(nested)]
    pub fraction: JustFraction,
}

impl DeviceSettings {
    pub fn rate(&self) -> f32 {
        self.fraction.numerator as f32 / self.fraction.denominator as f32
    }
    pub fn ffmpeg_r(&self) -> String {
        format!("{}/{}", self.fraction.denominator, self.fraction.numerator)
    }
}
