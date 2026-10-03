//! A five-field cron schedule shared by configuration validation and workers.
use chrono::{DateTime, Datelike, Timelike, Utc};
use chrono_tz::Tz;

#[derive(Debug, Clone)]
pub struct CronSchedule {
    fields: [u64; 5],
    day_of_month_wildcard: bool,
    day_of_week_wildcard: bool,
    zone: Tz,
}
impl CronSchedule {
    pub fn parse(expression: &str, time_zone: &str) -> Result<Self, &'static str> {
        let parts: Vec<_> = expression.split_whitespace().collect();
        if parts.len() != 5 {
            return Err("Schedule requires a five-field cron expression.");
        }
        let zone = time_zone
            .parse()
            .map_err(|_| "Schedule time zone is invalid.")?;
        let mut fields = [0; 5];
        for (index, (min, max)) in [(0, 59), (0, 23), (1, 31), (1, 12), (0, 7)]
            .into_iter()
            .enumerate()
        {
            fields[index] = parse_field(parts[index], min, max)?;
        }
        Ok(Self {
            fields,
            day_of_month_wildcard: parts[2].starts_with('*'),
            day_of_week_wildcard: parts[4].starts_with('*'),
            zone,
        })
    }
    pub fn is_due(&self, now: DateTime<Utc>) -> bool {
        let local = now.with_timezone(&self.zone);
        let matches = |i: usize, value: u32| self.fields[i] & (1 << value) != 0;
        let dom = matches(2, local.day());
        let weekday = local.weekday().num_days_from_sunday();
        let dow = matches(4, weekday) || (weekday == 0 && matches(4, 7));
        let day = if !self.day_of_month_wildcard && !self.day_of_week_wildcard {
            dom || dow
        } else {
            dom && dow
        };
        matches(0, local.minute()) && matches(1, local.hour()) && matches(3, local.month()) && day
    }
}
fn parse_field(field: &str, minimum: u32, maximum: u32) -> Result<u64, &'static str> {
    const ERROR: &str = "Cron field contains an invalid value, range, or step.";
    let mut mask = 0;
    for part in field.split(',') {
        let (range, step) = match part.split_once('/') {
            Some((range, step)) => (range, step.parse::<u32>().map_err(|_| ERROR)?),
            None => (part, 1),
        };
        if step == 0 {
            return Err(ERROR);
        }
        let (start, end) = if range == "*" {
            (minimum, maximum)
        } else if let Some((start, end)) = range.split_once('-') {
            (
                start.parse::<u32>().map_err(|_| ERROR)?,
                end.parse::<u32>().map_err(|_| ERROR)?,
            )
        } else {
            let start = range.parse::<u32>().map_err(|_| ERROR)?;
            (start, if part.contains('/') { maximum } else { start })
        };
        if start < minimum || end > maximum || start > end {
            return Err(ERROR);
        }
        for value in start..=end {
            if (value - start).is_multiple_of(step) {
                mask |= 1 << value;
            }
        }
    }
    Ok(mask)
}
/// Stored invalid schedules are inert; mutations must call `CronSchedule::parse`.
pub fn schedule_is_due(expression: Option<&str>, time_zone: &str, now: DateTime<Utc>) -> bool {
    expression
        .and_then(|value| CronSchedule::parse(value, time_zone).ok())
        .is_some_and(|schedule| schedule.is_due(now))
}
#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    #[test]
    fn rejects_every_invalid_list_member_and_timezone() {
        for value in [
            "0,99 * * * *",
            "0,bad * * * *",
            "0, * * * *",
            "*/0 * * * *",
            "5-1 * * * *",
            "* * *",
        ] {
            assert!(CronSchedule::parse(value, "UTC").is_err(), "{value}");
        }
        assert!(CronSchedule::parse("* * * * *", "invalid-zone").is_err());
    }
    #[test]
    fn ranges_steps_sunday_and_local_time_share_validation_and_execution() {
        let now = Utc.with_ymd_and_hms(2026, 9, 29, 8, 30, 0).unwrap();
        assert!(
            CronSchedule::parse("30 10 * * 2", "Europe/Paris")
                .unwrap()
                .is_due(now)
        );
        assert!(
            CronSchedule::parse("0/15 8-10 * * 1,2", "UTC")
                .unwrap()
                .is_due(now)
        );
        assert!(
            !CronSchedule::parse("31 10 * * *", "Europe/Paris")
                .unwrap()
                .is_due(now)
        );
        let sunday = Utc.with_ymd_and_hms(2026, 9, 27, 8, 30, 0).unwrap();
        for day in ["0", "7"] {
            assert!(
                CronSchedule::parse(&format!("* * * * {day}"), "UTC")
                    .unwrap()
                    .is_due(sunday)
            );
        }
    }
}
