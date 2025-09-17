use anyhow::Result;
use v4l::{Device, capability::Flags};

#[derive(Debug, Clone)]
pub struct DeviceInfo {
    pub index: usize,
    pub path: String,
    pub name: String,
    pub driver: String,
    pub capabilities: Flags,
}

impl DeviceInfo {
    pub fn device(&self) -> Result<Device> {
        Device::new(self.index).map_err(|e| {
            anyhow::format_err!(
                "Failed to open {}: {}. Is your webcam connected?",
                self.path,
                e
            )
        })
    }
}

impl std::fmt::Display for DeviceInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{} - {} ({})", self.path, self.name, self.driver)
    }
}
