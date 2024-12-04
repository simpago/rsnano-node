pub struct AsyncRuntime {
    pub tokio: tokio::runtime::Runtime,
}

impl AsyncRuntime {
    pub fn new(tokio: tokio::runtime::Runtime) -> Self {
        Self { tokio }
    }

    pub fn post<F>(&self, action: F)
    where
        F: FnOnce() + Send + 'static,
    {
        self.tokio.spawn_blocking(action);
    }
}

impl Default for AsyncRuntime {
    fn default() -> Self {
        let tokio = tokio::runtime::Builder::new_multi_thread()
            .thread_name("tokio runtime")
            .enable_all()
            .build()
            .unwrap();
        AsyncRuntime::new(tokio)
    }
}
