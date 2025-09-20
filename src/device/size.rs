use confique::Config;
use std::fmt::Display;

#[derive(Config, Debug, Clone)]
pub struct JustFrameSize {
    pub width: u32,
    pub height: u32,
}

impl Display for JustFrameSize {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&format!("{}x{}", self.width, self.height))
    }
}
