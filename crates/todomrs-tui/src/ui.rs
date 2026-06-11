use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
    Frame,
};

use crate::app::{App, InputMode, View};

/// TokyoNight-compatible highlight color (blue-gray selection)
const HIGHLIGHT_BG: Color = Color::Rgb(55, 68, 100);
/// Slightly lighter than default DarkGray for completed task visibility
const COMPLETED_FG: Color = Color::Indexed(244);

pub fn draw(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(0)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(3),
            Constraint::Length(1),
        ])
        .split(f.size());

    let main_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(20), Constraint::Percentage(80)])
        .split(chunks[0]);

    draw_sidebar(f, app, main_chunks[0]);
    draw_main_content(f, app, main_chunks[1]);
    draw_input_field(f, app, chunks[1]);
    draw_status_bar(f, app, chunks[2]);

    if app.show_help {
        draw_help(f);
    }
}

fn draw_sidebar(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    // Split sidebar into Views section and Projects section
    let project_count = app.project_counts.len();
    let projects_height = if project_count > 0 {
        project_count as u16 + 2 // items + border
    } else {
        2 // just the border
    };

    let sidebar_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(8), // 6 view items + border (2)
            Constraint::Min(projects_height),
        ])
        .split(area);

    // Views section
    let view_items = vec![
        ListItem::new("Inbox"),
        ListItem::new("Today"),
        ListItem::new("Upcoming"),
        ListItem::new("Projects"),
        ListItem::new("Completed"),
        ListItem::new("Recurring"),
    ];

    let view_selected = match app.current_view {
        View::Inbox => 0,
        View::Today => 1,
        View::Upcoming => 2,
        View::Projects => 3,
        View::Completed => 4,
        View::Recurring => 5,
    };

    let view_list = List::new(view_items)
        .block(Block::default().borders(Borders::ALL).title("Views"))
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .bg(HIGHLIGHT_BG)
                .add_modifier(Modifier::BOLD),
        );

    let mut view_state =
        ratatui::widgets::ListState::default().with_selected(Some(view_selected));
    f.render_stateful_widget(view_list, sidebar_chunks[0], &mut view_state);

    // Projects section
    let project_items: Vec<ListItem> = app
        .project_counts
        .iter()
        .map(|(id, name, pending, completed)| {
            let is_selected = app.selected_project_id == Some(*id);
            let label = if *pending > 0 && *completed > 0 {
                format!("{} ({}/{})", name, pending, completed)
            } else if *pending > 0 {
                format!("{} ({})", name, pending)
            } else if *completed > 0 {
                format!("{} (✓{})", name, completed)
            } else {
                format!("{}", name)
            };
            let label = if is_selected {
                format!("▸ {}", label)
            } else {
                format!("  {}", label)
            };
            if is_selected {
                ListItem::new(label).style(
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                )
            } else {
                ListItem::new(label)
            }
        })
        .collect();

    let project_list = List::new(project_items)
        .block(Block::default().borders(Borders::ALL).title("Projects"));

    f.render_widget(project_list, sidebar_chunks[1]);
}

fn draw_main_content(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    // ── Project Browser Mode ──────────────────────────────────
    if app.is_browsing_projects() {
        draw_project_browser(f, app, area);
        return;
    }

    // ── Project filter indicator (any view) ────────────────────
    let project_filter = if app.selected_project_id.is_some() {
        app.project_counts
            .iter()
            .find(|(id, _, _, _)| Some(*id) == app.selected_project_id)
            .map(|(_, name, _, _)| name.clone())
    } else {
        None
    };

    let base_title = match app.current_view {
        View::Inbox => "Inbox",
        View::Today => "Today",
        View::Upcoming => "Upcoming",
        View::Projects => "Projects",
        View::Completed => "Completed",
        View::Recurring => "Recurring",
    };

    let title = if app.current_view == View::Projects {
        // In Projects view, show just the project name when filtering
        if let Some(ref proj_name) = project_filter {
            format!("Project: {}", proj_name)
        } else {
            base_title.to_string()
        }
    } else if let Some(ref proj_name) = project_filter {
        format!("{} [project: {}]", base_title, proj_name)
    } else if !app.search_query.is_empty() && app.input_mode == InputMode::Normal {
        format!("{} [search: {}]", base_title, app.search_query)
    } else {
        base_title.to_string()
    };

    let filtered = app.filtered_tasks();

    if filtered.is_empty() {
        let placeholder = if !app.search_query.is_empty() {
            "No matching tasks."
        } else if app.current_view == View::Completed {
            "No completed tasks."
        } else if project_filter.is_some() {
            "No tasks in this project."
        } else if app.current_view == View::Projects {
            "No projects yet. Press 'a' to create one."
        } else if app.current_view == View::Recurring {
            "No recurring tasks. Add one with 'every day' etc."
        } else {
            "No tasks. Press 'a' to add one."
        };
        let content = Paragraph::new(placeholder)
            .style(Style::default().fg(Color::DarkGray))
            .block(Block::default().borders(Borders::ALL).title(title.as_str()));
        f.render_widget(content, area);
        return;
    }

    let items: Vec<ListItem> = filtered
        .iter()
        .map(|task| {
            let priority_indicator = match task.priority {
                todomrs_core::domain::Priority::Urgent => "!!! ",
                todomrs_core::domain::Priority::High => "!! ",
                todomrs_core::domain::Priority::Medium => "! ",
                _ => "",
            };

            let status_icon = if task.status == todomrs_core::domain::TaskStatus::Completed {
                "✓ "
            } else if task.is_overdue() {
                "⚠ "
            } else {
                "□ "
            };

            // Get recurrence indicator if task has a recurrence rule
            let recurrence_indicator = if let Some(rule_id) = task.recurrence_rule_id {
                if let Some(rule) = app.recurrence_rules.get(&rule_id) {
                    let interval_str = if rule.interval == 1 {
                        String::new()
                    } else {
                        format!("{}", rule.interval)
                    };
                    let kind_str = match rule.kind {
                        todomrs_core::domain::RecurrenceKind::Daily => format!("{}d", interval_str),
                        todomrs_core::domain::RecurrenceKind::Weekly => format!("{}w", interval_str),
                        todomrs_core::domain::RecurrenceKind::Monthly => format!("{}m", interval_str),
                        todomrs_core::domain::RecurrenceKind::Yearly => format!("{}y", interval_str),
                    };
                    format!("♻{} ", kind_str)
                } else {
                    String::new()
                }
            } else {
                String::new()
            };

            let due_str = task
                .due_at
                .map(|dt| {
                    let date = dt.format("%d/%m");
                    let midnight = dt.naive_utc().date().and_hms_opt(0, 0, 0).unwrap();
                    let midnight_dt: chrono::DateTime<chrono::Utc> =
                        chrono::DateTime::from_naive_utc_and_offset(midnight, chrono::Utc);
                    if dt != midnight_dt {
                        format!("{} {}", date, dt.format("%H:%M"))
                    } else {
                        date.to_string()
                    }
                })
                .unwrap_or_default();

            let suffix = if due_str.is_empty() {
                String::new()
            } else {
                format!(" [{}]", due_str)
            };
            let full_text =
                format!("{}{}{}{}{}", status_icon, priority_indicator, recurrence_indicator, task.title, suffix);

            if task.status == todomrs_core::domain::TaskStatus::Completed {
                let title_and_suffix = format!("{}{}", task.title, suffix);
                ListItem::new(Line::from(vec![
                    ratatui::text::Span::raw(format!("{}{}{}", status_icon, priority_indicator, recurrence_indicator)),
                    ratatui::text::Span::styled(
                        title_and_suffix,
                        Style::default()
                            .fg(COMPLETED_FG)
                            .add_modifier(Modifier::CROSSED_OUT),
                    ),
                ]))
            } else if task.is_overdue() {
                ListItem::new(full_text).style(Style::default().fg(Color::Red))
            } else {
                ListItem::new(full_text)
            }
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!("{} ({})", title, filtered.len())),
        )
        .highlight_style(Style::default().bg(HIGHLIGHT_BG).fg(Color::White));

    let mut state =
        ratatui::widgets::ListState::default().with_selected(Some(app.selected_index));
    f.render_stateful_widget(list, area, &mut state);
}

/// Render the project browser in the main content area.
fn draw_project_browser(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    if app.project_counts.is_empty() {
        let content = Paragraph::new("No projects yet. Press 'a' to create one.")
            .style(Style::default().fg(Color::DarkGray))
            .block(Block::default().borders(Borders::ALL).title("Projects"));
        f.render_widget(content, area);
        return;
    }

    let items: Vec<ListItem> = app
        .project_counts
        .iter()
        .map(|(_, name, pending, completed)| {
            let desc = if *pending > 0 && *completed > 0 {
                format!("{} pending, {} done", pending, completed)
            } else if *pending > 0 {
                format!("{} pending", pending)
            } else if *completed > 0 {
                format!("{} done", completed)
            } else {
                "empty".to_string()
            };
            ListItem::new(format!("{}  —  {}", name, desc))
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!("Projects ({})", app.project_counts.len())),
        )
        .highlight_style(Style::default().bg(HIGHLIGHT_BG).fg(Color::White));

    let mut state = ratatui::widgets::ListState::default()
        .with_selected(Some(app.project_selected_index));
    f.render_stateful_widget(list, area, &mut state);
}

fn draw_input_field(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let title = match app.input_mode {
        InputMode::Normal => {
            if app.current_view == View::Projects && app.is_browsing_projects() {
                "Press 'a' to add project, Enter on a project to filter"
            } else if app.current_view == View::Projects {
                "Press 'a' to add task, Esc to clear filter"
            } else if app.selected_project_id.is_some() {
                "Press 'a' to add task, Esc to clear project filter"
            } else {
                "Press 'a' to add task"
            }
        }
        InputMode::Editing => {
            if app.current_view == View::Projects {
                "Add project (Enter to save, Esc to cancel)"
            } else {
                "Add task (Enter to save, Esc to cancel)"
            }
        }
        InputMode::EditingTask(_) => "Edit task (Enter to save, Esc to cancel)",
        InputMode::Searching => {
            if app.search_query.is_empty() {
                "Search (type query, Enter to confirm, Esc to cancel)"
            } else {
                "Search active (Esc to clear)"
            }
        }
    };

    let display_text = match app.input_mode {
        InputMode::Searching => &app.search_query,
        _ => &app.input_buffer,
    };

    let input = Paragraph::new(display_text.as_str())
        .style(match app.input_mode {
            InputMode::Normal => Style::default().fg(Color::DarkGray),
            InputMode::Editing | InputMode::EditingTask(_) | InputMode::Searching => {
                Style::default().fg(Color::White)
            }
        })
        .block(Block::default().borders(Borders::ALL).title(title));

    f.render_widget(input, area);

    // Render cursor position for any editing mode
    if matches!(
        app.input_mode,
        InputMode::Editing | InputMode::EditingTask(_) | InputMode::Searching
    ) {
        let display_text = match app.input_mode {
            InputMode::Searching => &app.search_query,
            _ => &app.input_buffer,
        };
        let cursor_pos = app.cursor_position.min(display_text.len());
        let cursor_x = area.x + (cursor_pos as u16).min(area.width.saturating_sub(2)) + 1;
        f.set_cursor(cursor_x, area.y + 1);
    }

    // Render status message if present
    if let Some(ref msg) = app.status_message {
        if app.input_mode == InputMode::Normal {
            let msg_width = msg.len() as u16;
            let msg_x = area.x + area.width.saturating_sub(msg_width + 2);
            if msg_x > area.x {
                let status =
                    Paragraph::new(msg.as_str()).style(Style::default().fg(Color::Green));
                f.render_widget(
                    status,
                    ratatui::layout::Rect {
                        x: msg_x,
                        y: area.y,
                        width: msg_width + 2,
                        height: 1,
                    },
                );
            }
        }
    }
}

fn draw_status_bar(f: &mut Frame, app: &App, area: ratatui::layout::Rect) {
    let view_name = match app.current_view {
        View::Inbox => "Inbox",
        View::Today => "Today",
        View::Upcoming => "Upcoming",
        View::Projects => "Projects",
        View::Completed => "Completed",
        View::Recurring => "Recurring",
    };

    let status = Line::from(vec![
        Span::styled(
            " TodoRS ",
            Style::default().bg(Color::Blue).fg(Color::White),
        ),
        Span::raw(format!(" {} ", view_name)),
        if app.selected_project_id.is_some() {
            Span::styled(
                "[P] ",
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            )
        } else {
            Span::raw("")
        },
        Span::raw("│ "),
        Span::styled("q", Style::default().fg(Color::Yellow)),
        Span::raw(" Quit "),
        Span::styled("?", Style::default().fg(Color::Yellow)),
        Span::raw(" Help "),
        Span::styled("a", Style::default().fg(Color::Yellow)),
        Span::raw(" Add "),
        Span::styled("e", Style::default().fg(Color::Yellow)),
        Span::raw(" Edit "),
        Span::styled("x", Style::default().fg(Color::Yellow)),
        Span::raw(" Toggle "),
        Span::styled("d", Style::default().fg(Color::Yellow)),
        Span::raw(" Del "),
        Span::styled("C", Style::default().fg(Color::Yellow)),
        Span::raw(" Clear "),
        Span::styled("/", Style::default().fg(Color::Yellow)),
        Span::raw(" Search "),
    ]);

    let paragraph = Paragraph::new(status).style(Style::default().bg(Color::DarkGray));
    f.render_widget(paragraph, area);
}

fn draw_help(f: &mut Frame) {
    let area = f.size();
    let help_h = (area.height / 2).min(area.height.saturating_sub(2)).max(34);
    let help_w = (area.width / 2).min(area.width.saturating_sub(4)).max(50);
    let help_area = ratatui::layout::Rect {
        x: (area.width.saturating_sub(help_w)) / 2,
        y: (area.height.saturating_sub(help_h)) / 2,
        width: help_w,
        height: help_h,
    };

    let help_text = vec![
        Line::from(Span::styled(
            "Keyboard Shortcuts",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from("Navigation:"),
        Line::from("  j/↓    — Next item"),
        Line::from("  k/↑    — Previous item"),
        Line::from("  1      — Inbox view"),
        Line::from("  2      — Today view"),
        Line::from("  3      — Upcoming view"),
        Line::from("  4      — Projects view"),
        Line::from("  5      — Completed view"),
        Line::from("  6      — Recurring view"),
        Line::from(""),
        Line::from("Task Operations:"),
        Line::from("  a      — Add task / Add project (in Projects view)"),
        Line::from("  e      — Edit task (re-parse title, date, priority, project)"),
        Line::from("  x      — Toggle complete"),
        Line::from("  d      — Delete task"),
        Line::from("  C      — Clear all completed"),
        Line::from(""),
        Line::from("Recurrence:"),
        Line::from("  every day/week/month  — Set recurrence"),
        Line::from("  wait! every day       — Wait for completion"),
        Line::from("  every! day            — Anchor to completion date"),
        Line::from(""),
        Line::from("Projects:"),
        Line::from("  Enter  — Select/deselect a project to filter by"),
        Line::from("  a      — Create project (in Projects view)"),
        Line::from("  d      — Delete project (in Projects view)"),
        Line::from(""),
        Line::from("Search & Help:"),
        Line::from("  /      — Search"),
        Line::from("  ?      — Toggle help"),
        Line::from("  q      — Quit"),
        Line::from(""),
        Line::from("Input Mode:"),
        Line::from("  ←/→     — Move cursor"),
        Line::from("  Backspace — Delete character"),
        Line::from("  Home    — Start of line"),
        Line::from("  End     — End of line"),
        Line::from("  Ctrl+A  — Start of line"),
        Line::from("  Ctrl+E  — End of line"),
        Line::from("  Ctrl+W  — Delete word (add mode)"),
        Line::from("  Enter   — Confirm"),
        Line::from("  Esc     — Cancel"),
    ];

    let paragraph = Paragraph::new(help_text)
        .block(Block::default().borders(Borders::ALL).title("Help"));

    f.render_widget(Clear, help_area);
    f.render_widget(paragraph, help_area);
}
