use std::path::Path;

pub trait Store {
    fn copy_db(&self, destination: &Path) -> anyhow::Result<()>;
}
