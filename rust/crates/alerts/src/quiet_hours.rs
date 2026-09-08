use chrono::{DateTime, Datelike, NaiveTime, Timelike, Utc};
use chrono_tz::Tz;
use serde_json::Value;

use crate::AlertError;

fn resolve_zone(name: &str) -> Option<Tz> {
    name.parse().ok().or_else(|| {
        crate::windows_time_zones::WINDOWS_ZONES
            .iter()
            .find(|(windows, _)| windows.eq_ignore_ascii_case(name))
            .and_then(|(_, iana)| iana.parse().ok())
    })
}

struct QuietHour {
    zone: Tz,
    start: NaiveTime,
    end: NaiveTime,
    day: Option<u32>,
}

impl QuietHour {
    fn parse(value: &Value) -> Option<Self> {
        let string = |lower, upper| {
            value
                .get(lower)
                .or_else(|| value.get(upper))
                .and_then(Value::as_str)
        };
        let time = |lower, upper| {
            let value = string(lower, upper)?;
            NaiveTime::parse_from_str(value, "%H:%M:%S%.f")
                .or_else(|_| NaiveTime::parse_from_str(value, "%H:%M"))
                .ok()
        };
        let kind =
            string("$type", "scheduleType").or_else(|| string("ScheduleType", "ScheduleType"))?;
        let day = match kind {
            "Daily" => None,
            "Weekly" => {
                let value = value.get("dayOfWeek").or_else(|| value.get("DayOfWeek"))?;
                const DAYS: [&str; 7] = [
                    "Sunday",
                    "Monday",
                    "Tuesday",
                    "Wednesday",
                    "Thursday",
                    "Friday",
                    "Saturday",
                ];
                Some(
                    value
                        .as_str()
                        .and_then(|name| {
                            DAYS.iter()
                                .position(|day| day.eq_ignore_ascii_case(name))
                                .map(|i| i as u32)
                        })
                        .or_else(|| value.as_u64().filter(|v| *v < 7).map(|v| v as u32))?,
                )
            }
            _ => return None,
        };
        Some(Self {
            zone: resolve_zone(string("timezone", "Timezone")?)?,
            start: time("startTime", "StartTime")?,
            end: time("endTime", "EndTime")?,
            day,
        })
    }

    fn contains(&self, now: DateTime<Utc>) -> bool {
        let local = now.with_timezone(&self.zone);
        let time = local.time();
        let overnight = self.start > self.end;
        let in_range = if overnight {
            time >= self.start || time <= self.end
        } else {
            time >= self.start && time <= self.end
        };
        let day = if overnight && time <= self.end {
            local.weekday().pred()
        } else {
            local.weekday()
        };
        in_range
            && self
                .day
                .is_none_or(|expected| expected == day.num_days_from_sunday())
    }

    fn overlaps(&self, other: &Self) -> bool {
        if self.zone != other.zone {
            return false;
        }
        const DAY: i64 = 86_400_000_000_000;
        let range = |hour: &Self, day: u32| {
            let ticks = |time: NaiveTime| {
                i64::from(time.num_seconds_from_midnight()) * 1_000_000_000
                    + i64::from(time.nanosecond())
            };
            let offset = i64::from(day) * DAY;
            (
                offset + ticks(hour.start),
                offset + ticks(hour.end) + if hour.start > hour.end { DAY } else { 0 },
            )
        };
        for left_day in 0..7 {
            if self.day.is_some_and(|day| day != left_day) {
                continue;
            }
            let (start, end) = range(self, left_day);
            for right_day in 0..7 {
                if other.day.is_some_and(|day| day != right_day) {
                    continue;
                }
                let (other_start, other_end) = range(other, right_day);
                for week in -1..=1 {
                    let offset = week * 7 * DAY;
                    if start <= other_end + offset && other_start + offset <= end {
                        return true;
                    }
                }
            }
        }
        false
    }
}

pub fn is_in_quiet_hours(values: &[Value], now: DateTime<Utc>) -> bool {
    values
        .iter()
        .filter_map(QuietHour::parse)
        .any(|hour| hour.contains(now))
}

pub(crate) fn validate(values: &[Value]) -> Result<(), AlertError> {
    let hours = values
        .iter()
        .map(|value| {
            QuietHour::parse(value).ok_or_else(|| {
                AlertError::Validation(
                    "Quiet hours require a valid schedule, timezone, time range and weekly day."
                        .into(),
                )
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    for (index, hour) in hours.iter().enumerate() {
        if hours[..index].iter().any(|other| hour.overlaps(other)) {
            return Err(AlertError::Validation("Quiet hours overlap.".into()));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    fn daily() -> Value {
        json!({"$type":"Daily","name":"Paris morning","startTime":"10:00:00","endTime":"11:00:00","timezone":"Europe/Paris"})
    }
    #[test]
    fn bundled_windows_mappings_resolve_without_os_timezone_data() {
        let mut names = std::collections::HashSet::new();
        for (windows, iana) in crate::windows_time_zones::WINDOWS_ZONES {
            assert!(
                names.insert(windows.to_ascii_lowercase()),
                "duplicate Windows ID: {windows}"
            );
            let target: Tz = iana
                .parse()
                .expect("CLDR target must exist in bundled chrono-tz");
            // UTC is also a valid native IANA input; retain native-ID precedence.
            let expected = windows.parse::<Tz>().unwrap_or(target);
            assert_eq!(resolve_zone(windows), Some(expected), "{windows}");
            assert_eq!(resolve_zone(&windows.to_ascii_lowercase()), Some(target));
        }
        assert_eq!(resolve_zone("UTC"), Some(chrono_tz::UTC));
        assert_eq!(
            resolve_zone("Eastern Standard Time"),
            Some(chrono_tz::America::New_York)
        );
        assert_eq!(
            resolve_zone("India Standard Time"),
            Some(chrono_tz::Asia::Calcutta)
        );
        assert!(resolve_zone("Not/A_Timezone").is_none());
    }
    fn weekly(day: &str, start: &str, end: &str) -> Value {
        json!({"$type":"Weekly","dayOfWeek":day,"startTime":start,"endTime":end,"timezone":"Europe/Paris"})
    }

    // AlertRuleQuietHourTests.IsInQuietHours_ShouldApplyIanaTimeZoneAndDaylightSavingOffset.
    #[test]
    fn applies_iana_timezone_and_summer_winter_offsets() {
        for time in ["2026-07-14T08:30:00Z", "2026-12-14T09:30:00Z"] {
            assert!(is_in_quiet_hours(&[daily()], time.parse().unwrap()));
        }
    }
    // IsInQuietHours_ShouldUsePreviousDayForWeeklyRangeCrossingMidnight.
    #[test]
    fn weekly_full_day_names_use_previous_day_after_midnight() {
        let hour = weekly("Sunday", "22:00:00", "02:00:00");
        assert!(is_in_quiet_hours(
            &[hour.clone()],
            "2026-07-19T23:00:00Z".parse().unwrap()
        ));
        assert!(!is_in_quiet_hours(
            &[hour],
            "2026-07-18T23:00:00Z".parse().unwrap()
        ));
    }
    // Constructor_ShouldRejectUnknownTimeZone.
    #[test]
    fn rejects_unknown_zones_malformed_times_and_invalid_weekly_days() {
        for (key, value) in [
            ("timezone", json!("Not/A_Timezone")),
            ("startTime", json!("garbage")),
            ("$type", json!("Monthly")),
        ] {
            let mut hour = daily();
            hour[key] = value;
            assert!(validate(&[hour]).is_err());
        }
        assert!(validate(&[weekly("Someday", "22:00:00", "02:00:00")]).is_err());
    }
    // AlertRuleQuietHourTests.Overlaps_ShouldNormalizeEquivalentIanaAndWindowsTimeZoneIds.
    #[test]
    fn windows_and_iana_quiet_hours_share_overlap_and_dst_rules() {
        let mut windows = daily();
        windows["timezone"] = json!("Romance Standard Time");
        windows["startTime"] = json!("10:30:00");
        windows["endTime"] = json!("11:30:00");
        let before = windows.clone();
        validate(std::slice::from_ref(&windows)).unwrap();
        assert!(
            matches!(validate(&[daily(), windows.clone()]), Err(AlertError::Validation(message)) if message == "Quiet hours overlap.")
        );
        for (instant, expected) in [
            ("2026-07-14T08:45:00Z", true),
            ("2026-12-14T09:45:00Z", true),
            ("2026-12-14T08:45:00Z", false),
        ] {
            assert_eq!(
                is_in_quiet_hours(std::slice::from_ref(&windows), instant.parse().unwrap()),
                expected
            );
        }
        assert_eq!(
            windows, before,
            "normalization must not rewrite the stored/public value"
        );
        windows["startTime"] = json!("12:00:00");
        windows["endTime"] = json!("13:00:00");
        assert!(validate(&[daily(), windows]).is_ok());
    }
    // Overlaps_ShouldDetectWeeklyOverlapOnFollowingDay and
    // Overlaps_ShouldNotTreatEarlyHoursAsPartOfSameDayOvernightRange.
    #[test]
    fn overlap_detection_respects_week_wrap_and_overnight_day_ownership() {
        let sunday = weekly("Sunday", "22:00:00", "02:00:00");
        assert!(validate(&[sunday.clone(), weekly("Monday", "01:00:00", "03:00:00")]).is_err());
        assert!(validate(&[sunday, weekly("Sunday", "01:00:00", "03:00:00")]).is_ok());
        assert!(
            validate(&[
                weekly("Saturday", "22:00:00", "02:00:00"),
                weekly("Sunday", "01:00:00", "03:00:00")
            ])
            .is_err()
        );
    }
    // Serialization_ShouldExcludeComputedTimeZoneInfo: parsing does not mutate
    // the persisted/public document or add platform-specific timezone objects.
    #[test]
    fn validation_and_evaluation_leave_public_json_unchanged() {
        let value = daily();
        let before = value.clone();
        validate(std::slice::from_ref(&value)).unwrap();
        is_in_quiet_hours(std::slice::from_ref(&value), Utc::now());
        assert_eq!(value, before);
        assert!(!value.to_string().contains("timeZoneInfo"));
    }
}
