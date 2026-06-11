use chrono::{DateTime, Duration, Months, Utc};
use uuid::Uuid;

use crate::domain::{RecurrenceKind, RecurrenceRule};

/// Stateless engine for computing recurrence occurrences.
pub struct RecurrenceEngine;

impl RecurrenceEngine {
    /// Compute the next occurrence of a recurrence rule from a given point.
    ///
    /// The `from` datetime is the anchor – typically the original due date or
    /// the previous occurrence.
    pub fn next_occurrence(rule: &RecurrenceRule, from: DateTime<Utc>) -> DateTime<Utc> {
        match rule.kind {
            RecurrenceKind::Daily => from + Duration::days(rule.interval as i64),
            RecurrenceKind::Weekly => from + Duration::days(7 * rule.interval as i64),
            RecurrenceKind::Monthly => {
                // checked_add_months safely handles year rollover (e.g. Dec → Jan)
                from.checked_add_months(Months::new(rule.interval as u32))
                    .unwrap_or(from)
            }
            RecurrenceKind::Yearly => from
                .checked_add_months(Months::new(12 * rule.interval as u32))
                .unwrap_or(from),
        }
    }

    /// Convenience factory for a daily recurrence rule.
    pub fn create_daily_rule(task_id: Uuid, interval: i32) -> RecurrenceRule {
        let now = Utc::now();
        RecurrenceRule {
            id: Uuid::new_v4(),
            task_id,
            kind: RecurrenceKind::Daily,
            interval,
            by_weekday: None,
            by_monthday: None,
            timezone: "UTC".to_string(),
            created_at: now,
            updated_at: now,
        }
    }

    /// Convenience factory for a weekly recurrence rule.
    pub fn create_weekly_rule(task_id: Uuid, interval: i32) -> RecurrenceRule {
        let now = Utc::now();
        RecurrenceRule {
            id: Uuid::new_v4(),
            task_id,
            kind: RecurrenceKind::Weekly,
            interval,
            by_weekday: None,
            by_monthday: None,
            timezone: "UTC".to_string(),
            created_at: now,
            updated_at: now,
        }
    }
}

// ── tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Datelike;

    fn make_rule(task_id: Uuid, kind: RecurrenceKind, interval: i32) -> RecurrenceRule {
        let now = Utc::now();
        RecurrenceRule {
            id: Uuid::new_v4(),
            task_id,
            kind,
            interval,
            by_weekday: None,
            by_monthday: None,
            timezone: "UTC".to_string(),
            created_at: now,
            updated_at: now,
        }
    }

    #[test]
    fn test_daily_recurrence() {
        let rule = make_rule(Uuid::new_v4(), RecurrenceKind::Daily, 1);
        let from = DateTime::from_timestamp(1_000_000_000, 0).unwrap();
        let next = RecurrenceEngine::next_occurrence(&rule, from);
        assert_eq!(next, from + Duration::days(1));
    }

    #[test]
    fn test_weekly_recurrence() {
        let rule = make_rule(Uuid::new_v4(), RecurrenceKind::Weekly, 1);
        let from = DateTime::from_timestamp(1_000_000_000, 0).unwrap();
        let next = RecurrenceEngine::next_occurrence(&rule, from);
        assert_eq!(next, from + Duration::days(7));
    }

    #[test]
    fn test_every_2_days() {
        let rule = make_rule(Uuid::new_v4(), RecurrenceKind::Daily, 2);
        let from = DateTime::from_timestamp(1_000_000_000, 0).unwrap();
        let next = RecurrenceEngine::next_occurrence(&rule, from);
        assert_eq!(next, from + Duration::days(2));
    }

    #[test]
    fn test_monthly_recurrence() {
        let rule = make_rule(Uuid::new_v4(), RecurrenceKind::Monthly, 1);
        let from = DateTime::from_timestamp(1_000_000_000, 0).unwrap();
        let next = RecurrenceEngine::next_occurrence(&rule, from);
        let expected = from.checked_add_months(Months::new(1)).unwrap();
        assert_eq!(next, expected);
    }

    #[test]
    fn test_yearly_recurrence() {
        let rule = make_rule(Uuid::new_v4(), RecurrenceKind::Yearly, 1);
        let from = DateTime::from_timestamp(1_000_000_000, 0).unwrap();
        let next = RecurrenceEngine::next_occurrence(&rule, from);
        let expected = from.checked_add_months(Months::new(12)).unwrap();
        assert_eq!(next, expected);
    }

    #[test]
    fn test_monthly_rollover() {
        // December + 1 month → January next year
        let dec = DateTime::from_timestamp(1_733_068_800, 0).unwrap(); // 2024-12-01T00:00:00Z
        assert_eq!(dec.month(), 12);
        assert_eq!(dec.year(), 2024);

        let rule = make_rule(Uuid::new_v4(), RecurrenceKind::Monthly, 1);
        let next = RecurrenceEngine::next_occurrence(&rule, dec);

        assert_eq!(next.month(), 1);
        assert_eq!(next.year(), 2025);
    }

    #[test]
    fn test_monthly_rollover_multiple_months() {
        // October + 3 months → January next year
        let oct = DateTime::from_timestamp(1_727_827_200, 0).unwrap(); // 2024-10-01T00:00:00Z
        assert_eq!(oct.month(), 10);

        let rule = make_rule(Uuid::new_v4(), RecurrenceKind::Monthly, 3);
        let next = RecurrenceEngine::next_occurrence(&rule, oct);

        assert_eq!(next.month(), 1);
        assert_eq!(next.year(), 2025);
    }

    #[test]
    fn test_create_daily_rule() {
        let task_id = Uuid::new_v4();
        let rule = RecurrenceEngine::create_daily_rule(task_id, 1);
        assert_eq!(rule.task_id, task_id);
        assert_eq!(rule.kind, RecurrenceKind::Daily);
        assert_eq!(rule.interval, 1);
    }

    #[test]
    fn test_create_weekly_rule() {
        let task_id = Uuid::new_v4();
        let rule = RecurrenceEngine::create_weekly_rule(task_id, 3);
        assert_eq!(rule.task_id, task_id);
        assert_eq!(rule.kind, RecurrenceKind::Weekly);
        assert_eq!(rule.interval, 3);
    }
}
