pub struct OpenclConfig {
    pub platform: u32,
    pub device: u32,
    pub threads: u32,
}

impl OpenclConfig {
    pub fn new() -> Self {
        Default::default()
    }
}

impl Default for OpenclConfig {
    fn default() -> Self {
        Self {
            platform: 0,
            device: 0,
            threads: 1024 * 1024,
        }
    }
}
