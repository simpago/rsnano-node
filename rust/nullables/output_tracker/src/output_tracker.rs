use std::{
    cell::RefCell,
    rc::{Rc, Weak},
};

pub struct OutputTracker<T: Clone + 'static> {
    output: RefCell<Vec<T>>,
}

impl<T: Clone + 'static> OutputTracker<T> {
    pub fn new() -> Self {
        Self {
            output: RefCell::new(Vec::new()),
        }
    }

    pub fn add(&self, t: T) {
        self.output.borrow_mut().push(t);
    }

    pub fn output(&self) -> Vec<T> {
        self.output.borrow().clone()
    }
}

pub struct OutputListener<T: Clone + 'static> {
    trackers: RefCell<Vec<Weak<OutputTracker<T>>>>,
}

impl<T: Clone + 'static> OutputListener<T> {
    pub fn new() -> Self {
        Self {
            trackers: RefCell::new(Vec::new()),
        }
    }

    pub fn track(&self) -> Rc<OutputTracker<T>> {
        let tracker = Rc::new(OutputTracker::new());
        self.trackers.borrow_mut().push(Rc::downgrade(&tracker));
        tracker
    }

    pub fn emit(&self, t: T) {
        let mut guard = self.trackers.borrow_mut();
        let mut should_clean = false;
        for tracker in guard.iter() {
            if let Some(tracker) = tracker.upgrade() {
                tracker.add(t.clone());
            } else {
                should_clean = true;
            }
        }

        if should_clean {
            guard.retain(|t| t.strong_count() > 0);
        }
    }

    pub fn is_tracked(&self) -> bool {
        self.tracker_count() > 0
    }

    pub fn tracker_count(&self) -> usize {
        self.trackers.borrow().len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn not_active_trackers() {
        let listener = OutputListener::new();
        listener.emit("foo");
    }

    #[test]
    fn track_one_output() {
        let listener = OutputListener::new();
        let tracker = listener.track();
        listener.emit("foo");
        assert_eq!(tracker.output(), vec!["foo"]);
    }

    #[test]
    fn track_multiple_outputs() {
        let listener = OutputListener::new();
        let tracker = listener.track();
        listener.emit("foo");
        listener.emit("bar");
        listener.emit("test");
        assert_eq!(tracker.output(), vec!["foo", "bar", "test"]);
    }

    #[test]
    fn multiple_trackers() {
        let listener = OutputListener::new();
        let tracker1 = listener.track();
        listener.emit("foo");
        let tracker2 = listener.track();
        listener.emit("bar");
        listener.emit("test");
        assert_eq!(tracker1.output(), vec!["foo", "bar", "test"]);
        assert_eq!(tracker2.output(), vec!["bar", "test"]);
    }

    #[test]
    fn stop_tracking_if_tracker_dropped() {
        let listener = OutputListener::new();
        let tracker = listener.track();
        listener.emit("foo");
        assert_eq!(listener.tracker_count(), 1);
        drop(tracker);
        listener.emit("bar");
        assert_eq!(listener.tracker_count(), 0);
    }
}
