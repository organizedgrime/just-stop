use std::fmt::Display;
use v4l::FourCC;

#[derive(Debug, Clone)]
pub struct JustFrameSize {
    pub fourcc: FourCC,
    pub width: u32,
    pub height: u32,
}
impl Display for JustFrameSize {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&format!("{}x{}", self.width, self.height))
    }
}
