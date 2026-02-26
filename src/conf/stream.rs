use confique::Config;
use serde::Serialize;

#[derive(Serialize, Config)]
pub struct JustStream {
    pub tcp: bool,
    pub port: usize,
}

impl std::fmt::Display for JustStream {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let protocol = if self.tcp { "tcp" } else { "udp" };
        write!(f, "{}://127.0.0.1:{}", protocol, &self.port)
    }
}
