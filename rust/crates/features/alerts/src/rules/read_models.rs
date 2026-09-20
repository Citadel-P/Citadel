use crate::*;

#[derive(Debug, Clone)]
pub struct AlertRuleListItem {
    pub rule: AlertRule,
    pub channels: Vec<AlertChannel>,
}
