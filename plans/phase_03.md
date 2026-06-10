# Phase 3: Natural Language Parser + Recurrence Engine

## Session Goal

Implement the natural language quick-add parser that converts human input like `pay electricity bill tomorrow 8pm +personal p1` into structured task data, and the recurrence rule engine that generates future occurrences of recurring tasks.

## Expected Outcome

- Natural language parser that extracts: title, due date/time, project, tags, priority, recurrence
- Parser handles common date expressions: today, tomorrow, next week, friday, etc.
- Recurrence engine that calculates next occurrence based on rule
- Comprehensive unit tests for parser and recurrence logic
- `cargo test` passes with all parser and recurrence tests

## Context

Phase 2 is complete. You have:
- Working store layer with CRUD operations
- Task, Project, Tag domain types
- SQLite database with all tables

Now you'll add the intelligence layer: parsing natural language input and handling recurring tasks.

## Prerequisites

- Phase 2 complete and committed
- All stores working
- Core domain types defined

## Tasks

### Task 1: Implement Natural Language Parser — Basic Structure

**Objective:** Create a parser that extracts project (+project), tags (@tag), and priority (p1-p4) from input.

**Steps:**

1. Create `crates/todomrs-core/src/parser.rs`:

```rust
use crate::domain::{Priority, Task};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq)]
pub struct ParsedTask {
    pub title: String,
    pub project: Option<String>,
    pub tags: Vec<String>,
    pub priority: Priority,
    pub due_date: Option<String>,
    pub due_time: Option<String>,
    pub recurrence: Option<String>,
}

pub struct NaturalLanguageParser;

impl NaturalLanguageParser {
    pub fn parse(input: &str) -> ParsedTask {
        let mut title_parts = Vec::new();
        let mut project = None;
        let mut tags = Vec::new();
        let mut priority = Priority::None;
        let mut due_date = None;
        let mut due_time = None;
        let mut recurrence = None;

        let words: Vec<&str> = input.split_whitespace().collect();
        let mut i = 0;

        while i < words.len() {
            let word = words[i];

            // Check for project (+project)
            if word.starts_with('+') && word.len() > 1 {
                project = Some(word[1..].to_string());
            }
            // Check for tag (@tag)
            else if word.starts_with('@') && word.len() > 1 {
                tags.push(word[1..].to_string());
            }
            // Check for priority (p1, p2, p3, p4)
            else if word.len() == 2 && word.starts_with('p') {
                priority = match word.chars().nth(1) {
                    Some('1') => Priority::Urgent,
                    Some('2') => Priority::High,
                    Some('3') => Priority::Medium,
                    Some('4') => Priority::Low,
                    _ => Priority::None,
                };
            }
            // Check for date expressions
            else if Self::is_date_expression(word) {
                due_date = Some(word.to_string());
            }
            // Check for time expressions (simple HH:MM or HHam/pm)
            else if Self::is_time_expression(word) {
                due_time = Some(word.to_string());
            }
            // Check for recurrence (every day, every week, etc.)
            else if word == "every" && i + 1 < words.len() {
                let period = words[i + 1];
                recurrence = Some(format!("every {}", period));
                i += 1; // Skip next word
            }
            // Otherwise, it's part of the title
            else {
                title_parts.push(word);
            }

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

    fn is_date_expression(word: &str) -> bool {
        matches!(
            word.to_lowercase().as_str(),
            "today" | "tomorrow" | "monday" | "tuesday" | "wednesday" | "thursday" | "friday" | "saturday" | "sunday"
        )
    }

    fn is_time_expression(word: &str) -> bool {
        // Simple check for time patterns like "8pm", "9am", "14:30"
        let lower = word.to_lowercase();
        lower.ends_with("am") || lower.ends_with("pm") || lower.contains(':')
    }
}
```

2. Update `crates/todomrs-core/src/lib.rs`:

```rust
pub mod domain;
pub mod parser;

pub use parser::{NaturalLanguageParser, ParsedTask};
```

3. Add tests to `crates/todomrs-core/src/parser.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_task() {
        let parsed = NaturalLanguageParser::parse("Buy groceries");
        assert_eq!(parsed.title, "Buy groceries");
        assert_eq!(parsed.priority, Priority::None);
        assert!(parsed.project.is_none());
        assert!(parsed.tags.is_empty());
    }

    #[test]
    fn test_parse_with_project() {
        let parsed = NaturalLanguageParser::parse("Submit report +work");
        assert_eq!(parsed.title, "Submit report");
        assert_eq!(parsed.project, Some("work".to_string()));
    }

    #[test]
    fn test_parse_with_tags() {
        let parsed = NaturalLanguageParser::parse("Call dentist @phone @urgent");
        assert_eq!(parsed.title, "Call dentist");
        assert_eq!(parsed.tags, vec!["phone", "urgent"]);
    }

    #[test]
    fn test_parse_with_priority() {
        let parsed = NaturalLanguageParser::parse("Fix critical bug p1");
        assert_eq!(parsed.title, "Fix critical bug");
        assert_eq!(parsed.priority, Priority::Urgent);
    }

    #[test]
    fn test_parse_with_date() {
        let parsed = NaturalLanguageParser::parse("Pay bills tomorrow");
        assert_eq!(parsed.title, "Pay bills");
        assert_eq!(parsed.due_date, Some("tomorrow".to_string()));
    }

    #[test]
    fn test_parse_complex() {
        let parsed = NaturalLanguageParser::parse("Submit assignment +vit @writing due:friday p2");
        assert_eq!(parsed.title, "Submit assignment");
        assert_eq!(parsed.project, Some("vit".to_string()));
        assert_eq!(parsed.tags, vec!["writing"]);
        assert_eq!(parsed.due_date, Some("friday".to_string()));
        assert_eq!(parsed.priority, Priority::High);
    }
}
```

4. Run tests:
```bash
cargo test -p todomrs-core
```

Expected: All parser tests pass.

**Commit:**
```bash
git add .
git commit -m "feat: implement natural language parser for project, tags, priority"
```

---

### Task 2: Implement Date/Time Resolution

**Objective:** Convert date expressions (today, tomorrow, friday) and time expressions (8pm, 14:30) into actual DateTime values.

**Steps:**

1. Add date resolution to `crates/todomrs-core/src/parser.rs`:

```rust
use chrono::{DateTime, Duration, NaiveTime, Utc, Weekday};

impl ParsedTask {
    pub fn resolve_datetime(&self) -> Option<DateTime<Utc>> {
        let date = self.resolve_date()?;
        let time = self.resolve_time().unwrap_or(NaiveTime::from_hms(0, 0, 0));
        
        Some(date.and_time(time).and_utc())
    }

    fn resolve_date(&self) -> Option<chrono::NaiveDate> {
        let date_str = self.due_date.as_ref()?.to_lowercase();
        let today = Utc::now().date_naive();

        match date_str.as_str() {
            "today" => Some(today),
            "tomorrow" => Some(today + Duration::days(1)),
            "monday" => Some(Self::next_weekday(today, Weekday::Mon)),
            "tuesday" => Some(Self::next_weekday(today, Weekday::Tue)),
            "wednesday" => Some(Self::next_weekday(today, Weekday::Wed)),
            "thursday" => Some(Self::next_weekday(today, Weekday::Thu)),
            "friday" => Some(Self::next_weekday(today, Weekday::Fri)),
            "saturday" => Some(Self::next_weekday(today, Weekday::Sat)),
            "sunday" => Some(Self::next_weekday(today, Weekday::Sun)),
            _ => None,
        }
    }

    fn resolve_time(&self) -> Option<NaiveTime> {
        let time_str = self.due_time.as_ref()?;
        let lower = time_str.to_lowercase();

        // Handle "8pm", "9am" format
        if lower.ends_with("am") || lower.ends_with("pm") {
            let is_pm = lower.ends_with("pm");
            let hour_str = &lower[..lower.len() - 2];
            if let Ok(hour) = hour_str.parse::<u32>() {
                let hour = if is_pm && hour != 12 { hour + 12 } else if !is_pm && hour == 12 { 0 } else { hour };
                return NaiveTime::from_hms_opt(hour, 0, 0);
            }
        }

        // Handle "14:30" format
        if lower.contains(':') {
            let parts: Vec<&str> = lower.split(':').collect();
            if parts.len() == 2 {
                if let (Ok(hour), Ok(minute)) = (parts[0].parse::<u32>(), parts[1].parse::<u32>()) {
                    return NaiveTime::from_hms_opt(hour, minute, 0);
                }
            }
        }

        None
    }

    fn next_weekday(from: chrono::NaiveDate, weekday: Weekday) -> chrono::NaiveDate {
        let days_until = (weekday.num_days_from_monday() as i64 - from.weekday().num_days_from_monday() as i64 + 7) % 7;
        let days_until = if days_until == 0 { 7 } else { days_until };
        from + Duration::days(days_until)
    }
}
```

2. Add tests for date/time resolution:

```rust
#[test]
    fn test_resolve_today() {
        let parsed = ParsedTask {
            title: "Test".to_string(),
            project: None,
            tags: vec![],
            priority: Priority::None,
            due_date: Some("today".to_string()),
            due_time: None,
            recurrence: None,
        };

        let datetime = parsed.resolve_datetime();
        assert!(datetime.is_some());
        
        let today = Utc::now().date_naive();
        assert_eq!(datetime.unwrap().date_naive(), today);
    }

    #[test]
    fn test_resolve_tomorrow() {
        let parsed = ParsedTask {
            title: "Test".to_string(),
            project: None,
            tags: vec![],
            priority: Priority::None,
            due_date: Some("tomorrow".to_string()),
            due_time: None,
            recurrence: None,
        };

        let datetime = parsed.resolve_datetime();
        assert!(datetime.is_some());
        
        let tomorrow = Utc::now().date_naive() + Duration::days(1);
        assert_eq!(datetime.unwrap().date_naive(), tomorrow);
    }

    #[test]
    fn test_resolve_time_pm() {
        let parsed = ParsedTask {
            title: "Test".to_string(),
            project: None,
            tags: vec![],
            priority: Priority::None,
            due_date: Some("today".to_string()),
            due_time: Some("8pm".to_string()),
            recurrence: None,
        };

        let datetime = parsed.resolve_datetime();
        assert!(datetime.is_some());
        assert_eq!(datetime.unwrap().hour(), 20);
    }
}
```

3. Run tests:
```bash
cargo test -p todomrs-core
```

Expected: All tests pass.

**Commit:**
```bash
git add .
git commit -m "feat: implement date and time resolution in parser"
```

---

### Task 3: Implement Recurrence Rule Engine

**Objective:** Create a recurrence engine that calculates the next occurrence of a recurring task.

**Steps:**

1. Create `crates/todomrs-core/src/recurrence.rs`:

```rust
use crate::domain::{RecurrenceKind, RecurrenceRule};
use chrono::{DateTime, Duration, Utc};

pub struct RecurrenceEngine;

impl RecurrenceEngine {
    pub fn next_occurrence(rule: &RecurrenceRule, from: DateTime<Utc>) -> DateTime<Utc> {
        match rule.kind {
            RecurrenceKind::Daily => from + Duration::days(rule.interval as i64),
            RecurrenceKind::Weekly => from + Duration::weeks(rule.interval as i64),
            RecurrenceKind::Monthly => {
                let month = from.month() as i32 + rule.interval;
                let year = from.year() + (month - 1) / 12;
                let month = ((month - 1) % 12 + 12) % 12 + 1;
                
                from.with_year(year)
                    .and_then(|dt| dt.with_month(month as u32))
                    .unwrap_or(from)
            }
            RecurrenceKind::Yearly => {
                let year = from.year() + rule.interval;
                from.with_year(year).unwrap_or(from)
            }
        }
    }

    pub fn create_daily_rule(task_id: uuid::Uuid, interval: i32) -> RecurrenceRule {
        let now = Utc::now();
        RecurrenceRule {
            id: uuid::Uuid::new_v4(),
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

    pub fn create_weekly_rule(task_id: uuid::Uuid, interval: i32) -> RecurrenceRule {
        let now = Utc::now();
        RecurrenceRule {
            id: uuid::Uuid::new_v4(),
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
```

2. Update `crates/todomrs-core/src/lib.rs`:

```rust
pub mod domain;
pub mod parser;
pub mod recurrence;

pub use parser::{NaturalLanguageParser, ParsedTask};
pub use recurrence::RecurrenceEngine;
```

3. Add tests to `crates/todomrs-core/src/recurrence.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::RecurrenceKind;
    use chrono::TimeZone;

    #[test]
    fn test_daily_recurrence() {
        let rule = RecurrenceRule {
            id: uuid::Uuid::new_v4(),
            task_id: uuid::Uuid::new_v4(),
            kind: RecurrenceKind::Daily,
            interval: 1,
            by_weekday: None,
            by_monthday: None,
            timezone: "UTC".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let from = Utc.with_ymd_and_hms(2026, 6, 10, 12, 0, 0).unwrap();
        let next = RecurrenceEngine::next_occurrence(&rule, from);

        assert_eq!(next, Utc.with_ymd_and_hms(2026, 6, 11, 12, 0, 0).unwrap());
    }

    #[test]
    fn test_weekly_recurrence() {
        let rule = RecurrenceRule {
            id: uuid::Uuid::new_v4(),
            task_id: uuid::Uuid::new_v4(),
            kind: RecurrenceKind::Weekly,
            interval: 1,
            by_weekday: None,
            by_monthday: None,
            timezone: "UTC".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let from = Utc.with_ymd_and_hms(2026, 6, 10, 12, 0, 0).unwrap();
        let next = RecurrenceEngine::next_occurrence(&rule, from);

        assert_eq!(next, Utc.with_ymd_and_hms(2026, 6, 17, 12, 0, 0).unwrap());
    }

    #[test]
    fn test_every_2_days() {
        let rule = RecurrenceRule {
            id: uuid::Uuid::new_v4(),
            task_id: uuid::Uuid::new_v4(),
            kind: RecurrenceKind::Daily,
            interval: 2,
            by_weekday: None,
            by_monthday: None,
            timezone: "UTC".to_string(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let from = Utc.with_ymd_and_hms(2026, 6, 10, 12, 0, 0).unwrap();
        let next = RecurrenceEngine::next_occurrence(&rule, from);

        assert_eq!(next, Utc.with_ymd_and_hms(2026, 6, 12, 12, 0, 0).unwrap());
    }
}
```

4. Run tests:
```bash
cargo test -p todomrs-core
```

Expected: All recurrence tests pass.

**Commit:**
```bash
git add .
git commit -m "feat: implement recurrence rule engine for daily/weekly/monthly/yearly"
```

---

### Task 4: Integrate Parser with Task Creation

**Objective:** Create a helper function that takes parsed input and creates a Task with resolved dates and recurrence.

**Steps:**

1. Add to `crates/todomrs-core/src/parser.rs`:

```rust
use crate::domain::{Task, RecurrenceRule};
use crate::recurrence::RecurrenceEngine;

impl NaturalLanguageParser {
    pub fn create_task_from_input(input: &str, user_id: uuid::Uuid) -> (Task, Option<RecurrenceRule>) {
        let parsed = Self::parse(input);
        let mut task = Task::new(user_id, parsed.title);
        
        task.priority = parsed.priority;
        task.due_at = parsed.resolve_datetime();
        
        let recurrence_rule = parsed.recurrence.as_ref().map(|rec| {
            let rule = if rec.contains("day") {
                RecurrenceEngine::create_daily_rule(task.id, 1)
            } else if rec.contains("week") {
                RecurrenceEngine::create_weekly_rule(task.id, 1)
            } else {
                RecurrenceEngine::create_daily_rule(task.id, 1) // Default
            };
            task.recurrence_rule_id = Some(rule.id);
            rule
        });
        
        (task, recurrence_rule)
    }
}
```

2. Add integration test:

```rust
#[test]
fn test_create_task_from_complex_input() {
    let user_id = uuid::Uuid::new_v4();
    let (task, recurrence) = NaturalLanguageParser::create_task_from_input(
        "Pay electricity bill tomorrow 8pm +personal p1 every week",
        user_id
    );
    
    assert_eq!(task.title, "Pay electricity bill");
    assert_eq!(task.priority, Priority::Urgent);
    assert!(task.due_at.is_some());
    assert!(recurrence.is_some());
}
```

3. Run tests:
```bash
cargo test -p todomrs-core
```

Expected: All tests pass.

**Commit:**
```bash
git add .
git commit -m "feat: integrate parser with task creation and recurrence"
```

---

## Verification

Run all checks:

```bash
cargo build
cargo test
cargo clippy
```

Expected:
- Build succeeds
- All parser and recurrence tests pass
- No critical clippy warnings

## Pitfalls

1. **Don't overcomplicate the parser.** Start simple. We can add more date formats later.

2. **Don't forget timezone handling.** All dates should be UTC internally.

3. **Don't skip recurrence edge cases.** Test month boundaries, leap years, etc.

4. **Don't hardcode date formats.** Use chrono's parsing functions.

## Handoff to Next Phase

Phase 4 will assume:
- Natural language parser works
- Recurrence engine calculates next occurrences
- Tasks can be created from natural language input

Phase 4 will implement the operation log system for sync.
