#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Tab {
    Messages,
    Peers,
    Queues,
}

impl Tab {
    pub fn name(&self) -> &'static str {
        match self {
            Tab::Messages => "Messages",
            Tab::Peers => "Peers",
            Tab::Queues => "Queues",
        }
    }
}

pub static TAB_ORDER: [Tab; 3] = [Tab::Peers, Tab::Messages, Tab::Queues];

pub(crate) struct TabBarViewModel {
    pub selected: Tab,
    pub tabs: Vec<TabViewModel>,
}

impl TabBarViewModel {
    pub(crate) fn new() -> Self {
        let mut tabs = create_tab_view_models();
        tabs[0].selected = true;
        let selected = tabs[0].value;
        Self { tabs, selected }
    }

    pub fn select(&mut self, tab: Tab) {
        for t in &mut self.tabs {
            t.selected = t.value == tab;
        }
        self.selected = tab;
    }

    pub fn selected_tab(&self) -> Tab {
        self.selected
    }
}

fn create_tab_view_models() -> Vec<TabViewModel> {
    TAB_ORDER.iter().map(|t| TabViewModel::from(*t)).collect()
}

pub(crate) struct TabViewModel {
    pub selected: bool,
    pub label: &'static str,
    pub value: Tab,
}

impl From<Tab> for TabViewModel {
    fn from(value: Tab) -> Self {
        Self {
            selected: false,
            label: value.name(),
            value,
        }
    }
}
