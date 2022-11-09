pub struct TcpChannels {}

impl TcpChannels {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for TcpChannels {
    fn default() -> Self {
        Self::new()
    }
}
