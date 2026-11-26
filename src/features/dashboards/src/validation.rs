use crate::{DashboardContent, DashboardError, Layout, WidgetSettings};
use std::collections::HashSet;

pub const MAX_WIDGETS: usize = 24;
pub const MAX_METRICS: usize = 8;

impl Layout {
    fn valid(self) -> bool {
        self.w >= 2
            && self.h >= 2
            && self.h <= 24
            && self.x.saturating_add(self.w) <= 12
            && self.y <= 512
    }
    fn overlaps(self, other: Self) -> bool {
        self.x < other.x + other.w
            && other.x < self.x + self.w
            && self.y < other.y + other.h
            && other.y < self.y + self.h
    }
}

fn valid_link(href: &str) -> bool {
    if href.len() > 2048
        || href.trim() != href
        || href.contains('\\')
        || href.chars().any(char::is_control)
    {
        return false;
    }
    if href.starts_with('/') {
        return !href.starts_with("//");
    }
    url::Url::parse(href).is_ok_and(|url| {
        matches!(url.scheme(), "https" | "http")
            && url.has_host()
            && url.username().is_empty()
            && url.password().is_none()
    })
}

impl DashboardContent {
    pub fn validate(&mut self) -> Result<(), DashboardError> {
        let invalid = |message: &str| DashboardError::Invalid(message.to_owned());
        self.name = self.name.trim().to_owned();
        let config = &mut self.configuration;
        if self.name.is_empty() || self.name.chars().count() > 128 || self.name.contains('\0') {
            return Err(invalid("Enter a dashboard name of 1 to 128 characters."));
        }
        if config.schema_version != 2 {
            return Err(invalid("This dashboard version cannot be edited."));
        }
        config.tags =
            citadel_tags::tag_filter_names(&config.tags).map_err(|e| invalid(&e.to_string()))?;
        if !matches!(config.range_minutes, 15 | 60 | 360 | 1440 | 2880 | 10080) {
            return Err(invalid("Choose a supported history range."));
        }
        if config.widgets.len() > MAX_WIDGETS {
            return Err(invalid("A dashboard can contain up to 24 widgets."));
        }
        let mut ids = HashSet::new();
        let mut metrics = 0;
        for (index, widget) in config.widgets.iter().enumerate() {
            if widget.id.is_nil()
                || !ids.insert(widget.id)
                || widget.title.contains('\0')
                || widget.title.trim().is_empty()
                || widget.title.chars().count() > 128
            {
                return Err(invalid(
                    "Each widget needs its own ID and a title of 1 to 128 characters.",
                ));
            }
            if !widget.layout.valid()
                || config.widgets[..index]
                    .iter()
                    .any(|other| widget.layout.overlaps(other.layout))
            {
                return Err(invalid(
                    "Widget positions must fit the grid without overlapping.",
                ));
            }
            match &widget.settings {
                WidgetSettings::FleetSummary { measures }
                    if measures.is_empty()
                        || measures.iter().collect::<HashSet<_>>().len() != measures.len() =>
                {
                    return Err(invalid("Select distinct fleet measures."));
                }
                WidgetSettings::NeedsAttention { limit, kinds, .. }
                    if !matches!(limit, 3 | 5 | 10)
                        || kinds.iter().collect::<HashSet<_>>().len() != kinds.len() =>
                {
                    return Err(invalid(
                        "Choose valid attention filters and 3, 5 or 10 rows.",
                    ));
                }
                WidgetSettings::Watchlist { pins }
                    if pins.is_empty()
                        || pins.len() > 8
                        || pins.iter().any(|p| p.id.is_nil())
                        || pins.iter().enumerate().any(|(i, p)| pins[..i].contains(p)) =>
                {
                    return Err(invalid("Choose one to eight distinct resources."));
                }
                WidgetSettings::Metrics { source, .. } => {
                    metrics += 1;
                    if source.id.is_nil() || metrics > MAX_METRICS {
                        return Err(invalid(
                            "Select a resource for each of up to eight metric widgets.",
                        ));
                    }
                }
                WidgetSettings::Notes { text, links }
                    if text.contains('\0')
                        || text.chars().count() > 2048
                        || links.len() > 8
                        || links.iter().any(|l| {
                            l.label.contains('\0')
                                || l.label.trim().is_empty()
                                || l.label.chars().count() > 128
                                || !valid_link(&l.href)
                        }) =>
                {
                    return Err(invalid("Check the note length and its links."));
                }
                WidgetSettings::OpenAlerts { limit }
                | WidgetSettings::Backups { limit }
                | WidgetSettings::RecentActivity { limit }
                | WidgetSettings::UpcomingSchedules { limit }
                    if !(1..=10).contains(limit) =>
                {
                    return Err(invalid("Choose between one and ten rows."));
                }
                _ => {}
            }
        }
        let bytes = serde_json::to_vec(config).map_err(|e| invalid(&e.to_string()))?;
        if bytes.len() > 65536 {
            return Err(invalid("Invalid or oversized dashboard configuration."));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{DashboardConfiguration, Widget};
    use uuid::Uuid;

    #[test]
    fn note_text_accepts_literal_escapes_and_rejects_null_characters() {
        let mut board = DashboardContent {
            name: " Board ".into(),
            configuration: DashboardConfiguration {
                schema_version: 2,
                tags: vec![],
                range_minutes: 60,
                widgets: vec![Widget {
                    id: Uuid::now_v7(),
                    title: "Reference".into(),
                    layout: Layout {
                        x: 0,
                        y: 0,
                        w: 4,
                        h: 3,
                    },
                    settings: WidgetSettings::Notes {
                        text: r"JSON escape \u0000".into(),
                        links: vec![],
                    },
                }],
            },
        };
        board.validate().unwrap();
        assert_eq!(board.name, "Board");
        board.configuration.widgets[0].settings = WidgetSettings::Notes {
            text: "a\0b".into(),
            links: vec![],
        };
        assert!(matches!(board.validate(), Err(DashboardError::Invalid(_))));
    }
}
