use chrono::{DateTime, Utc};
use chrono::{Datelike, Timelike};
use chrono_tz::Tz;

pub(crate) fn cron_is_due(expression: Option<&str>, time_zone: &str, now: DateTime<Utc>) -> bool {
    let Some(expression) = expression else {
        return false;
    };
    let fields = expression.split_whitespace().collect::<Vec<_>>();
    if fields.len() != 5 {
        return false;
    }
    let Ok(zone) = time_zone.parse::<Tz>() else {
        return false;
    };
    let local = now.with_timezone(&zone);
    let day_of_month = cron_matches(fields[2], local.day(), 1, 31);
    let week_day = local.weekday().num_days_from_sunday();
    let day_of_week = cron_matches(fields[4], week_day, 0, 7)
        || (week_day == 0 && cron_matches(fields[4], 7, 0, 7));
    let day = if fields[2] != "*" && fields[4] != "*" {
        day_of_month || day_of_week
    } else {
        day_of_month && day_of_week
    };
    cron_matches(fields[0], local.minute(), 0, 59)
        && cron_matches(fields[1], local.hour(), 0, 23)
        && cron_matches(fields[3], local.month(), 1, 12)
        && day
}

fn cron_matches(field: &str, value: u32, minimum: u32, maximum: u32) -> bool {
    field.split(',').any(|part| {
        let (range, step) = part.split_once('/').map_or((part, 1), |(range, step)| {
            (range, step.parse().unwrap_or(0))
        });
        if step == 0 {
            return false;
        }
        let bounds = if range == "*" {
            Some((minimum, maximum))
        } else if let Some((start, end)) = range.split_once('-') {
            start.parse().ok().zip(end.parse().ok())
        } else {
            range.parse().ok().map(|single| (single, single))
        };
        bounds.is_some_and(|(start, end)| {
            start >= minimum
                && end <= maximum
                && start <= value
                && value <= end
                && (value - start).is_multiple_of(step)
        })
    })
}
