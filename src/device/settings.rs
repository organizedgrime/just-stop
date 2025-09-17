use crate::device::JustFrameSize;
use v4l::{FourCC, Fraction};

#[derive(Debug, Clone)]
pub struct DeviceSettings {
    // Codec
    pub format: FourCC,
    // Frame size
    pub size: JustFrameSize,
    // Framerate
    pub fraction: Fraction,
}
impl DeviceSettings {
    pub fn ffmpeg_r(&self) -> String {
        format!("{}/{}", self.fraction.denominator, self.fraction.numerator)
    }
}
