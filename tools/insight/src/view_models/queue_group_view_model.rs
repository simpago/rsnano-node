use num_format::{Locale, ToFormattedString};
use rsnano_core::utils::{FairQueueInfo, QueueInfo};
use std::{fmt::Debug, hash::Hash};
use strum::IntoEnumIterator;

use super::PaletteColor;

pub(crate) struct QueueGroupViewModel {
    pub heading: String,
    pub queues: Vec<QueueViewModel>,
}

impl QueueGroupViewModel {
    pub fn for_fair_queue<T, H>(heading: H, info: &FairQueueInfo<T>) -> Self
    where
        T: Clone + Hash + Eq + IntoEnumIterator + Debug,
        H: Into<String>,
    {
        let mut queues = Vec::new();

        for source in T::iter() {
            let info = info
                .queues
                .get(&source)
                .cloned()
                .unwrap_or_else(|| QueueInfo {
                    source,
                    size: 0,
                    max_size: 0,
                });
            let label = format!("{:?}", info.source);
            queues.push(QueueViewModel::new(label, info.size, info.max_size));
        }

        queues.push(QueueViewModel::new(
            "Total",
            info.total_size,
            info.total_max_size,
        ));

        QueueGroupViewModel {
            heading: heading.into(),
            queues,
        }
    }
}

pub(crate) struct QueueViewModel {
    pub label: String,
    pub value: String,
    pub max: String,
    pub progress: f32,
    pub color: PaletteColor,
}

impl QueueViewModel {
    pub fn new(label: impl Into<String>, value: usize, max: usize) -> Self {
        let progress = value as f32 / max as f32;
        QueueViewModel {
            label: label.into(),
            value: value.to_formatted_string(&Locale::en),
            max: max.to_formatted_string(&Locale::en),
            progress,
            color: match progress {
                0.75.. => PaletteColor::Red1,
                0.5..0.75 => PaletteColor::Orange1,
                _ => PaletteColor::Blue1,
            },
        }
    }
}
