use chrono::{DateTime, Datelike, Duration, NaiveDate, NaiveTime, Utc, Weekday};
use uuid::Uuid;

use crate::domain::{Priority, RecurrenceKind, RecurrenceRule, Task};

/// Result of parsing a natural language quick-add string.
#[derive(Debug, Clone, PartialEq)]
pub struct ParsedTask {
    /// The extracted title (everything not recognized as a marker).
    pub title: String,
    /// Project name extracted from `+project`.
    pub project: Option<String>,
    /// Tags extracted from `@tag` (multiple allowed).
    pub tags: Vec<String>,
    /// Priority extracted from `p1`–`p4`.
    pub priority: Priority,
    /// Date expression from `due:` prefix or raw date word.
    pub due_date: Option<String>,
    /// Time expression from time words (e.g. `8pm`, `14:30`).
    pub due_time: Option<String>,
    /// Recurrence expression from `every ...` patterns.
    pub recurrence: Option<String>,
}

/// Parses natural-language quick-add input into structured task fields.
pub struct NaturalLanguageParser;

impl NaturalLanguageParser {
    /// Parse a quick-add input string into its constituent parts.
    ///
    /// Recognised markers:
    /// - `+project` → project
    /// - `@tag`     → tags (multiple allowed)
    /// - `p1`–`p4`  → priority (p1=Urgent … p4=Low)
    /// - `due:expr` → due_date (value after colon)
    /// - `today`, `tomorrow`, weekday names → due_date
    /// - `8pm`, `9am`, `14:30` → due_time
    /// - `every day/week/month/year [N …]` → recurrence
    ///
    /// All other tokens are collected into the title.
    pub fn parse(input: &str) -> ParsedTask {
        let mut title_parts: Vec<String> = Vec::new();
        let mut project: Option<String> = None;
        let mut tags: Vec<String> = Vec::new();
        let mut priority: Priority = Priority::None;
        let mut due_date: Option<String> = None;
        let mut due_time: Option<String> = None;
        let mut recurrence: Option<String> = None;

        let words: Vec<&str> = input.split_whitespace().collect();
        let mut i = 0;
        while i < words.len() {
            let word = words[i];

            // ── due:prefix ──────────────────────────────────────────
            if let Some(pos) = word.find(':') {
                if pos > 0 && word[..pos].eq_ignore_ascii_case("due") {
                    let rest = word[pos + 1..].to_string();
                    if !rest.is_empty() {
                        due_date = Some(rest);
                    }
                    i += 1;
                    continue;
                }
            }

            // ── +project ────────────────────────────────────────────
            if word.starts_with('+') && word.len() > 1 {
                project = Some(word[1..].to_string());
                i += 1;
                continue;
            }

            // ── @tag ────────────────────────────────────────────────
            if word.starts_with('@') && word.len() > 1 {
                tags.push(word[1..].to_string());
                i += 1;
                continue;
            }

            // ── priority p1–p4 ──────────────────────────────────────
            if word.len() == 2 && word.starts_with('p') {
                let prio = match word.to_lowercase().as_str() {
                    "p1" => Some(Priority::Urgent),
                    "p2" => Some(Priority::High),
                    "p3" => Some(Priority::Medium),
                    "p4" => Some(Priority::Low),
                    _ => None,
                };
                if let Some(p) = prio {
                    priority = p;
                    i += 1;
                    continue;
                }
                // p0, p5, … fall through to title
            }

            // ── every … recurrence (two or three tokens) ────────────
            if word.eq_ignore_ascii_case("every") && i + 1 < words.len() {
                let next = words[i + 1];
                if is_period_word(next) {
                    recurrence = Some(format!("{} {}", word, next));
                    i += 2;
                    continue;
                }
                // every N days/weeks/months/years
                if i + 2 < words.len() {
                    if let Ok(n) = next.parse::<i32>() {
                        if n > 0 && is_period_word(words[i + 2]) {
                            recurrence = Some(format!("{} {} {}", word, next, words[i + 2]));
                            i += 3;
                            continue;
                        }
                    }
                }
            }

            // ── raw date words ──────────────────────────────────────
            if is_date_word(word) {
                due_date = Some(word.to_lowercase());
                i += 1;
                continue;
            }

            // ── time expressions ────────────────────────────────────
            if is_time_word(word) {
                due_time = Some(word.to_lowercase());
                i += 1;
                continue;
            }

            // ── everything else → title ─────────────────────────────
            title_parts.push(word.to_string());
            i += 1;
        }

        ParsedTask {
            title: title_parts.join(" "),
            project,
            tags,
            priority,
            due_date,
            due_time,
            recurrence,
        }
    }

    /// Parse input and immediately create a `Task` (and optional `RecurrenceRule`).
    pub fn create_task_from_input(input: &str, user_id: Uuid) -> (Task, Option<RecurrenceRule>) {
        let parsed = Self::parse(input);

        let due_at = parsed.resolve_datetime();
        let priority = parsed.priority;
        let mut task = Task::new(user_id, parsed.title);
        task.priority = priority;
        task.due_at = due_at;

        let recurrence_rule = parsed.recurrence.as_ref().and_then(|rec| {
            let words: Vec<&str> = rec.split_whitespace().collect();
            if words.len() < 2 || !words[0].eq_ignore_ascii_case("every") {
                return None;
            }

            let (interval, period) = if words.len() == 2 {
                (1, words[1])
            } else if words.len() == 3 {
                (words[1].parse::<i32>().ok()?, words[2])
            } else {
                return None;
            };

            let kind = match period.to_lowercase().as_str() {
                "day" | "days" => Some(RecurrenceKind::Daily),
                "week" | "weeks" => Some(RecurrenceKind::Weekly),
                "month" | "months" => Some(RecurrenceKind::Monthly),
                "year" | "years" => Some(RecurrenceKind::Yearly),
                _ => None,
            }?;

            let now = Utc::now();
            Some(RecurrenceRule {
                id: Uuid::new_v4(),
                task_id: task.id,
                kind,
                interval,
                by_weekday: None,
                by_monthday: None,
                timezone: "UTC".to_string(),
                created_at: now,
                updated_at: now,
            })
        });

        if let Some(ref rule) = recurrence_rule {
            task.recurrence_rule_id = Some(rule.id);
        }

        (task, recurrence_rule)
    }
}

// ── private helpers ─────────────────────────────────────────────────────

fn is_period_word(word: &str) -> bool {
    matches!(
        word.to_lowercase().as_str(),
        "day" | "days" | "week" | "weeks" | "month" | "months" | "year" | "years"
    )
}

fn is_date_word(word: &str) -> bool {
    matches!(
        word.to_lowercase().as_str(),
        "today"
            | "tomorrow"
            | "monday"
            | "tuesday"
            | "wednesday"
            | "thursday"
            | "friday"
            | "saturday"
            | "sunday"
    )
}

/// Recognise time expressions like `8pm`, `9am`, `14:30`.
fn is_time_word(word: &str) -> bool {
    let lower = word.to_lowercase();

    // pattern: <digits>am or <digits>pm
    if lower.len() >= 2 {
        let (num_part, suffix) = lower.split_at(lower.len() - 2);
        if suffix == "am" || suffix == "pm" {
            if let Ok(hour) = num_part.parse::<u32>() {
                if (1..=12).contains(&hour) {
                    return true;
                }
            }
        }
    }

    // pattern: HH:MM (24-hour)
    if let Some(pos) = lower.find(':') {
        if let (Ok(h), Ok(m)) = (lower[..pos].parse::<u32>(), lower[pos + 1..].parse::<u32>()) {
            if h < 24 && m < 60 {
                return true;
            }
        }
    }

    false
}

// ── date/time resolution ────────────────────────────────────────────────

impl ParsedTask {
    /// Resolve the due date string to an actual `NaiveDate`.
    ///
    /// Supports `today`, `tomorrow`, and weekday names.
    /// Weekday names always resolve to the *next* occurrence (≥7 days if today).
    pub fn resolve_date(&self) -> Option<NaiveDate> {
        let today = Utc::now().naive_utc().date();
        match self.due_date.as_deref()? {
            "today" => Some(today),
            "tomorrow" => Some(today + Duration::days(1)),
            d => {
                let weekday = match d.to_lowercase().as_str() {
                    "monday" => Weekday::Mon,
                    "tuesday" => Weekday::Tue,
                    "wednesday" => Weekday::Wed,
                    "thursday" => Weekday::Thu,
                    "friday" => Weekday::Fri,
                    "saturday" => Weekday::Sat,
                    "sunday" => Weekday::Sun,
                    _ => return None,
                };
                Some(Self::next_weekday(today, weekday))
            }
        }
    }

    /// Resolve a time expression to an actual `NaiveTime`.
    ///
    /// Supports `8pm`, `9am`, `14:30` formats.
    pub fn resolve_time(&self) -> Option<NaiveTime> {
        let time_str = self.due_time.as_deref()?;
        let lower = time_str.to_lowercase();

        // <digits>am / <digits>pm
        if lower.len() >= 2 {
            let (num_part, suffix) = lower.split_at(lower.len() - 2);
            if suffix == "am" || suffix == "pm" {
                if let Ok(hour) = num_part.parse::<u32>() {
                    let hour_24 = match (suffix, hour) {
                        ("am", 12) => 0,
                        ("pm", 12) => 12,
                        ("pm", h) => h + 12,
                        _ => hour,
                    };
                    return NaiveTime::from_hms_opt(hour_24, 0, 0);
                }
            }
        }

        // HH:MM 24-hour
        if let Some(pos) = lower.find(':') {
            let hour = lower[..pos].parse::<u32>().ok()?;
            let minute = lower[pos + 1..].parse::<u32>().ok()?;
            if hour < 24 && minute < 60 {
                return NaiveTime::from_hms_opt(hour, minute, 0);
            }
        }

        None
    }

    /// Combine resolved date and time into a `DateTime<Utc>`.
    ///
    /// * Date + time present → combined.
    /// * Date only            → time is midnight.
    /// * Time only            → date is today.
    pub fn resolve_datetime(&self) -> Option<DateTime<Utc>> {
        let date = self.resolve_date();
        let time = self.resolve_time();

        match (date, time) {
            (Some(d), Some(t)) => Some(d.and_time(t).and_utc()),
            (Some(d), None) => Some(d.and_time(NaiveTime::from_hms_opt(0, 0, 0).expect("midnight is always valid")).and_utc()),
            (None, Some(t)) => {
                let today = Utc::now().naive_utc().date();
                Some(today.and_time(t).and_utc())
            }
            (None, None) => None,
        }
    }

    /// Return the next occurrence of `weekday` on or after `from`.
    ///
    /// If `from` is already the target weekday, returns **7 days later**
    /// (i.e. next week, not today).
    pub fn next_weekday(from: NaiveDate, weekday: Weekday) -> NaiveDate {
        let from_weekday = from.weekday();
        let target_day = weekday.num_days_from_monday() as i32;
        let current_day = from_weekday.num_days_from_monday() as i32;

        if target_day == current_day {
            return from + Duration::days(7);
        }

        let days_ahead = (target_day - current_day + 7) % 7;
        from + Duration::days(days_ahead as i64)
    }
}

// ── tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Datelike;

    // ── parse ──────────────────────────────────────────────────────────

    #[test]
    fn test_parse_simple_task() {
        let p = NaturalLanguageParser::parse("Buy groceries");
        assert_eq!(p.title, "Buy groceries");
        assert_eq!(p.project, None);
        assert!(p.tags.is_empty());
        assert_eq!(p.priority, Priority::None);
        assert_eq!(p.due_date, None);
        assert_eq!(p.due_time, None);
        assert_eq!(p.recurrence, None);
    }

    #[test]
    fn test_parse_with_project() {
        let p = NaturalLanguageParser::parse("Task +project");
        assert_eq!(p.title, "Task");
        assert_eq!(p.project, Some("project".to_string()));
    }

    #[test]
    fn test_parse_with_tags() {
        let p = NaturalLanguageParser::parse("Task @tag");
        assert_eq!(p.title, "Task");
        assert_eq!(p.tags, vec!["tag"]);
    }

    #[test]
    fn test_parse_with_priority() {
        let p = NaturalLanguageParser::parse("Task p2");
        assert_eq!(p.title, "Task");
        assert_eq!(p.priority, Priority::High);
    }

    #[test]
    fn test_parse_with_date() {
        let p = NaturalLanguageParser::parse("Task tomorrow");
        assert_eq!(p.title, "Task");
        assert_eq!(p.due_date, Some("tomorrow".to_string()));
    }

    #[test]
    fn test_parse_due_prefix() {
        let p = NaturalLanguageParser::parse("Task due:friday");
        assert_eq!(p.title, "Task");
        assert_eq!(p.due_date, Some("friday".to_string()));
    }

    #[test]
    fn test_parse_with_time() {
        let p = NaturalLanguageParser::parse("Task 8pm");
        assert_eq!(p.title, "Task");
        assert_eq!(p.due_time, Some("8pm".to_string()));
    }

    #[test]
    fn test_parse_time_24h() {
        let p = NaturalLanguageParser::parse("Task 14:30");
        assert_eq!(p.title, "Task");
        assert_eq!(p.due_time, Some("14:30".to_string()));
    }

    #[test]
    fn test_parse_recurrence() {
        let p = NaturalLanguageParser::parse("Task every week");
        assert_eq!(p.title, "Task");
        assert_eq!(p.recurrence, Some("every week".to_string()));
    }

    #[test]
    fn test_parse_complex() {
        let input = "Submit assignment +vit @writing due:friday p2";
        let p = NaturalLanguageParser::parse(input);
        assert_eq!(p.title, "Submit assignment");
        assert_eq!(p.project, Some("vit".to_string()));
        assert_eq!(p.tags, vec!["writing"]);
        assert_eq!(p.priority, Priority::High);
        assert_eq!(p.due_date, Some("friday".to_string()));
        assert_eq!(p.recurrence, None);
    }

    #[test]
    fn test_empty_input() {
        let p = NaturalLanguageParser::parse("");
        assert_eq!(p.title, "");
        assert_eq!(p.project, None);
        assert!(p.tags.is_empty());
        assert_eq!(p.priority, Priority::None);
    }

    #[test]
    fn test_only_markers() {
        let p = NaturalLanguageParser::parse("+project @tag p1");
        assert_eq!(p.title, "");
        assert_eq!(p.project, Some("project".to_string()));
        assert_eq!(p.tags, vec!["tag"]);
        assert_eq!(p.priority, Priority::Urgent);
    }

    #[test]
    fn test_p5_not_priority() {
        let p = NaturalLanguageParser::parse("Task p5");
        assert_eq!(p.title, "Task p5");
        assert_eq!(p.priority, Priority::None);
    }

    #[test]
    fn test_multiple_tags() {
        let p = NaturalLanguageParser::parse("Task @home @work @urgent");
        assert_eq!(p.title, "Task");
        assert_eq!(p.tags, vec!["home", "work", "urgent"]);
    }

    #[test]
    fn test_due_today_prefix() {
        let p = NaturalLanguageParser::parse("due:today");
        assert_eq!(p.title, "");
        assert_eq!(p.due_date, Some("today".to_string()));
    }

    // ── resolve_date / resolve_time / resolve_datetime ─────────────────

    #[test]
    fn test_resolve_today() {
        let p = ParsedTask {
            title: String::new(),
            project: None,
            tags: vec![],
            priority: Priority::None,
            due_date: Some("today".to_string()),
            due_time: None,
            recurrence: None,
        };
        let expected = Utc::now().naive_utc().date();
        assert_eq!(p.resolve_date(), Some(expected));
    }

    #[test]
    fn test_resolve_tomorrow() {
        let p = ParsedTask {
            title: String::new(),
            project: None,
            tags: vec![],
            priority: Priority::None,
            due_date: Some("tomorrow".to_string()),
            due_time: None,
            recurrence: None,
        };
        let expected = Utc::now().naive_utc().date() + Duration::days(1);
        assert_eq!(p.resolve_date(), Some(expected));
    }

    #[test]
    fn test_resolve_weekday() {
        // Next Monday — should always be in the future and be Monday
        let p = ParsedTask {
            title: String::new(),
            project: None,
            tags: vec![],
            priority: Priority::None,
            due_date: Some("monday".to_string()),
            due_time: None,
            recurrence: None,
        };
        let date = p.resolve_date().expect("should resolve");
        assert_eq!(date.weekday(), Weekday::Mon);
        assert!(date > Utc::now().naive_utc().date() || date == Utc::now().naive_utc().date());
    }

    #[test]
    fn test_resolve_time_pm() {
        let p = ParsedTask {
            title: String::new(),
            project: None,
            tags: vec![],
            priority: Priority::None,
            due_date: None,
            due_time: Some("8pm".to_string()),
            recurrence: None,
        };
        assert_eq!(p.resolve_time(), NaiveTime::from_hms_opt(20, 0, 0));
    }

    #[test]
    fn test_resolve_time_am() {
        let p = ParsedTask {
            title: String::new(),
            project: None,
            tags: vec![],
            priority: Priority::None,
            due_date: None,
            due_time: Some("9am".to_string()),
            recurrence: None,
        };
        assert_eq!(p.resolve_time(), NaiveTime::from_hms_opt(9, 0, 0));
    }

    #[test]
    fn test_resolve_time_12am() {
        let p = ParsedTask {
            title: String::new(),
            project: None,
            tags: vec![],
            priority: Priority::None,
            due_date: None,
            due_time: Some("12am".to_string()),
            recurrence: None,
        };
        assert_eq!(p.resolve_time(), NaiveTime::from_hms_opt(0, 0, 0));
    }

    #[test]
    fn test_resolve_time_12pm() {
        let p = ParsedTask {
            title: String::new(),
            project: None,
            tags: vec![],
            priority: Priority::None,
            due_date: None,
            due_time: Some("12pm".to_string()),
            recurrence: None,
        };
        assert_eq!(p.resolve_time(), NaiveTime::from_hms_opt(12, 0, 0));
    }

    #[test]
    fn test_resolve_datetime_today() {
        let p = ParsedTask {
            title: String::new(),
            project: None,
            tags: vec![],
            priority: Priority::None,
            due_date: Some("today".to_string()),
            due_time: None,
            recurrence: None,
        };
        let dt = p.resolve_datetime().expect("should resolve");
        let today = Utc::now().naive_utc().date();
        assert_eq!(dt.naive_utc().date(), today);
        assert_eq!(dt.naive_utc().time(), NaiveTime::from_hms_opt(0, 0, 0).unwrap());
    }

    #[test]
    fn test_resolve_time_only() {
        let p = ParsedTask {
            title: String::new(),
            project: None,
            tags: vec![],
            priority: Priority::None,
            due_date: None,
            due_time: Some("14:30".to_string()),
            recurrence: None,
        };
        let dt = p.resolve_datetime().expect("should resolve");
        let today = Utc::now().naive_utc().date();
        assert_eq!(dt.naive_utc().date(), today);
        assert_eq!(dt.naive_utc().time(), NaiveTime::from_hms_opt(14, 30, 0).unwrap());
    }

    // ── next_weekday ───────────────────────────────────────────────────

    #[test]
    fn test_next_weekday_same_day_is_next_week() {
        // Thursday 2026-06-11 is a Thursday
        let date = NaiveDate::from_ymd_opt(2026, 6, 11).unwrap();
        assert_eq!(date.weekday(), Weekday::Thu);
        let next = ParsedTask::next_weekday(date, Weekday::Thu);
        // Should be 7 days later = Thursday 2026-06-18
        assert_eq!(next, NaiveDate::from_ymd_opt(2026, 6, 18).unwrap());
    }

    #[test]
    fn test_next_weekday_next_day() {
        let date = NaiveDate::from_ymd_opt(2026, 6, 11).unwrap(); // Thursday
        let next = ParsedTask::next_weekday(date, Weekday::Fri);
        assert_eq!(next, NaiveDate::from_ymd_opt(2026, 6, 12).unwrap());
    }

    #[test]
    fn test_next_weekday_wrap_around() {
        let date = NaiveDate::from_ymd_opt(2026, 6, 11).unwrap(); // Thursday
        let next = ParsedTask::next_weekday(date, Weekday::Wed);
        assert_eq!(next, NaiveDate::from_ymd_opt(2026, 6, 17).unwrap());
    }

    // ── create_task_from_input ─────────────────────────────────────────

    #[test]
    fn test_create_task_from_complex_input() {
        let user_id = Uuid::new_v4();
        let input = "Submit assignment +vit @writing due:friday p2 every week";
        let (task, rule) = NaturalLanguageParser::create_task_from_input(input, user_id);

        assert_eq!(task.title, "Submit assignment");
        assert_eq!(task.user_id, user_id);
        assert_eq!(task.priority, Priority::High);
        assert!(task.due_at.is_some());
        // Should be a Friday
        assert_eq!(
            task.due_at.unwrap().naive_utc().date().weekday(),
            Weekday::Fri
        );

        let rule = rule.expect("recurrence rule should be created");
        assert_eq!(rule.task_id, task.id);
        assert_eq!(rule.kind, RecurrenceKind::Weekly);
        assert_eq!(rule.interval, 1);
        assert_eq!(task.recurrence_rule_id, Some(rule.id));
    }

    #[test]
    fn test_next_weekday_same_day_returns_seven_days() {
        // Monday -> next Monday should be 7 days
        let date = NaiveDate::from_ymd_opt(2026, 6, 1).unwrap(); // Monday
        assert_eq!(date.weekday(), Weekday::Mon);
        let next = ParsedTask::next_weekday(date, Weekday::Mon);
        assert_eq!(next, NaiveDate::from_ymd_opt(2026, 6, 8).unwrap());
    }

    #[test]
    fn test_next_weekday_this_week() {
        // Monday -> Friday = 4 days (same week)
        let monday = NaiveDate::from_ymd_opt(2026, 6, 1).unwrap();
        let friday = ParsedTask::next_weekday(monday, Weekday::Fri);
        assert_eq!(friday, NaiveDate::from_ymd_opt(2026, 6, 5).unwrap());

        // Monday -> Wednesday = 2 days
        let wednesday = ParsedTask::next_weekday(monday, Weekday::Wed);
        assert_eq!(wednesday, NaiveDate::from_ymd_opt(2026, 6, 3).unwrap());
    }

    #[test]
    fn test_create_task_simple() {
        let user_id = Uuid::new_v4();
        let (task, rule) = NaturalLanguageParser::create_task_from_input("Buy milk", user_id);
        assert_eq!(task.title, "Buy milk");
        assert_eq!(task.priority, Priority::None);
        assert!(task.due_at.is_none());
        assert!(rule.is_none());
        assert!(task.recurrence_rule_id.is_none());
    }

    #[test]
    fn test_create_task_with_every_2_days() {
        let user_id = Uuid::new_v4();
        let (task, rule) =
            NaturalLanguageParser::create_task_from_input("Water plants every 2 days", user_id);
        assert_eq!(task.title, "Water plants");
        let rule = rule.expect("recurrence rule should be created");
        assert_eq!(rule.kind, RecurrenceKind::Daily);
        assert_eq!(rule.interval, 2);
    }
}
