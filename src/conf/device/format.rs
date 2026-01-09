use v4l::FourCC;

pub struct JustFormatDescription(pub v4l::format::Description);
impl JustFormatDescription {
    pub fn description(&self) -> String {
        self.0.description.clone()
    }
    pub fn fourcc(&self) -> FourCC {
        self.0.fourcc
    }
}
impl std::fmt::Display for JustFormatDescription {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.description())
    }
}
