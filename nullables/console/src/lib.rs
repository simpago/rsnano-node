use rsnano_output_tracker::{OutputListenerMt, OutputTrackerMt};
use std::sync::Arc;

pub struct Console {
    is_null: bool,
    output_listener: OutputListenerMt<String>,
}

impl Console {
    fn new(is_null: bool) -> Self {
        Self {
            is_null,
            output_listener: OutputListenerMt::new(),
        }
    }
    pub fn new_null() -> Self {
        Self::new(true)
    }

    pub fn println(&self, line: impl AsRef<str>) {
        let line = line.as_ref();
        if self.output_listener.is_tracked() {
            self.output_listener.emit(line.to_owned());
        }
        if !self.is_null {
            println!("{}", line);
        }
    }

    pub fn track(&self) -> Arc<OutputTrackerMt<String>> {
        self.output_listener.track()
    }
}

impl Default for Console {
    fn default() -> Self {
        Self::new(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_real_console() {
        assert_eq!(Console::default().is_null, false);
    }

    #[test]
    fn create_nulled_console() {
        assert_eq!(Console::new_null().is_null, true);
    }

    #[test]
    fn println() {
        let console = Console::new_null();
        let tracker = console.track();
        console.println("hello");
        let output = tracker.output();
        assert_eq!(output, ["hello"])
    }
}
