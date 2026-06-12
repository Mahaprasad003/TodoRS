use std::collections::HashMap;
use std::time::Instant;

use anyhow::Result;
use chrono::{Datelike, NaiveTime, Timelike, Weekday};
use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers};
use todomrs_core::domain::{AnchorMode, Priority, Project, RecurrenceRule, Task, TaskStatus};
use crate::notifications;
use todomrs_core::RecurrenceEngine;
use todomrs_core::NaturalLanguageParser;
use todomrs_store::{OperationStore, ProjectStore, RecurrenceRuleStore, TaskStore};
use todomrs_sync::operations::{Entity, Operation, OperationPayload, OperationType};
use todomrs_sync::SyncClient;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq)]
pub enum View {
    Inbox,
    Today,
    Upcoming,
    Projects,
    Completed,
    Recurring,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SyncStatus {
    Disabled,
    Syncing,
    Synced,
    Offline(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum InputMode {
    Normal,
    Editing,
    EditingTask(Uuid),
    Searching,
}

#[allow(dead_code)]
pub struct App {
    pub should_quit: bool,
    pub current_view: View,
    pub selected_index: usize,
    pub tasks: Vec<Task>,
    pub projects: Vec<Project>,
    pub user_id: Uuid,
    pub device_id: Uuid,
    pub show_help: bool,
    pub input_mode: InputMode,
    pub input_buffer: String,
    pub cursor_position: usize,
    pub search_query: String,
    pub previous_view: Option<View>,
    pub task_store: TaskStore,
    pub op_store: OperationStore,
    pub project_store: ProjectStore,
    pub recurrence_store: RecurrenceRuleStore,
    pub recurrence_rules: HashMap<Uuid, RecurrenceRule>,  // keyed by rule.id
    pub status_message: Option<String>,
    pub project_counts: Vec<(Uuid, String, usize, usize)>,
    pub selected_project_id: Option<Uuid>,
    pub project_selected_index: usize,
    pub sync_client: Option<SyncClient>,
    pub sync_status: SyncStatus,
    pub last_synced_at: chrono::DateTime<chrono::Utc>,
    pub last_sync_attempt: Instant,
    /// Timestamp of the most recent mutation (for debounce).
    pub last_mutation_at: Instant,
    /// Set after a mutation to trigger a debounced auto-sync (~10s later).
    pub sync_debounce_requested: bool,
}

impl std::fmt::Debug for App {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("App")
            .field("should_quit", &self.should_quit)
            .field("current_view", &self.current_view)
            .field("selected_index", &self.selected_index)
            .field("tasks", &self.tasks)
            .field("projects", &self.projects)
            .field("user_id", &self.user_id)
            .field("device_id", &self.device_id)
            .field("show_help", &self.show_help)
            .field("input_mode", &self.input_mode)
            .field("input_buffer", &self.input_buffer)
            .field("search_query", &self.search_query)
            .field("status_message", &self.status_message)
            .finish()
    }
}

impl App {
    pub fn new(
        user_id: Uuid,
        device_id: Uuid,
        task_store: TaskStore,
        op_store: OperationStore,
        project_store: ProjectStore,
        recurrence_store: RecurrenceRuleStore,
    ) -> Self {
        Self {
            should_quit: false,
            current_view: View::Inbox,
            selected_index: 0,
            tasks: Vec::new(),
            projects: Vec::new(),
            user_id,
            device_id,
            show_help: false,
            input_mode: InputMode::Normal,
            input_buffer: String::new(),
            cursor_position: 0,
            search_query: String::new(),
            previous_view: None,
            task_store,
            op_store,
            project_store,
            recurrence_store,
            recurrence_rules: HashMap::new(),
            status_message: None,
            project_counts: Vec::new(),
            selected_project_id: None,
            project_selected_index: 0,
            sync_client: None,
            sync_status: SyncStatus::Disabled,
            last_synced_at: chrono::DateTime::from_timestamp(0, 0).unwrap_or(chrono::Utc::now()),
            last_sync_attempt: Instant::now(),
            last_mutation_at: Instant::now(),
            sync_debounce_requested: false,
        }
    }

    /// Inject a SyncClient after construction (called from main.rs).
    pub fn set_sync_client(&mut self, client: SyncClient) {
        let is_auth = client.is_authenticated();
        self.sync_client = Some(client);
        self.sync_status = if is_auth {
            SyncStatus::Synced
        } else {
            SyncStatus::Disabled
        };
    }

    /// Load tasks from the database for the current user.
    pub async fn refresh_tasks(&mut self) -> Result<()> {
        self.tasks = self.task_store.get_all(self.user_id).await?;
        self.refresh_project_counts().await?;
        
        // Load all recurrence rules — keyed by rule.id, not task_id
        let rules = self.recurrence_store.get_all().await?;
        self.recurrence_rules = rules.into_iter().map(|r| (r.id, r)).collect();

        // Clamp selection to valid range
        let count = self.filtered_tasks().len();
        if count > 0 && self.selected_index >= count {
            self.selected_index = count.saturating_sub(1);
        }
        Ok(())
    }

    /// Filter tasks based on the current view.
    pub fn filtered_tasks(&self) -> Vec<&Task> {
        let today = chrono::Utc::now().date_naive();
        let mut tasks: Vec<&Task> = match self.current_view {
            View::Inbox => self.tasks.iter().collect(),
            View::Today => self
                .tasks
                .iter()
                .filter(|t| t.due_at.map(|dt| dt.date_naive() == today).unwrap_or(false))
                .collect(),
            View::Upcoming => self
                .tasks
                .iter()
                .filter(|t| t.due_at.map(|dt| dt.date_naive() > today).unwrap_or(false))
                .collect(),
            View::Projects => {
                // Return all tasks when filtering by a project, empty when browsing
                if self.selected_project_id.is_some() {
                    self.tasks.iter().collect()
                } else {
                    Vec::new()
                }
            }
            View::Completed => self
                .tasks
                .iter()
                .filter(|t| t.status == TaskStatus::Completed && t.deleted_at.is_none())
                .collect(),
            View::Recurring => self
                .tasks
                .iter()
                .filter(|t| t.recurrence_rule_id.is_some() && t.deleted_at.is_none())
                .collect(),
        };

        // Apply project filter if selected (even on non-Projects views)
        if let Some(proj_id) = self.selected_project_id {
            tasks.retain(|t| t.project_id == Some(proj_id));
        }

        // Apply search filter if active
        if !self.search_query.is_empty() {
            let query = self.search_query.to_lowercase();
            tasks.retain(|t| t.title.to_lowercase().contains(&query));
        }

        tasks
    }

    /// Handle one crossterm event.
    pub async fn handle_event(&mut self, event: Event) -> Result<()> {
        if let Event::Key(key) = event {
            if key.kind != KeyEventKind::Press {
                return Ok(());
            }

            match self.input_mode {
                InputMode::Normal => {
                    if self.show_help {
                        if let KeyCode::Char('?') | KeyCode::Esc = key.code {
                            self.show_help = false;
                        }
                        return Ok(());
                    }

                    match key.code {
                        KeyCode::Char('q') if key.modifiers.is_empty() => {
                            self.should_quit = true
                        }
                        KeyCode::Char('?') if key.modifiers.is_empty() => {
                            self.show_help = true
                        }
                        KeyCode::Char('a') if key.modifiers.is_empty() => {
                            if self.current_view == View::Projects {
                                self.add_project().await?;
                            } else {
                                self.input_mode = InputMode::Editing;
                                self.input_buffer.clear();
                                self.cursor_position = 0;
                            }
                        }
                        KeyCode::Char('e') if key.modifiers.is_empty() => {
                            let task_info = {
                                let filtered = self.filtered_tasks();
                                if self.selected_index < filtered.len() {
                                    let task = filtered[self.selected_index];
                                    Some((task.id, self.task_to_edit_string(task)))
                                } else {
                                    None
                                }
                            };
                            if let Some((id, edit_str)) = task_info {
                                self.input_buffer = edit_str;
                                self.cursor_position = self.input_buffer.len();
                                self.input_mode = InputMode::EditingTask(id);
                            }
                        }
                        KeyCode::Char('/') if key.modifiers.is_empty() => {
                            self.previous_view = Some(self.current_view.clone());
                            self.input_mode = InputMode::Searching;
                            self.search_query.clear();
                            self.cursor_position = 0;
                        }
                        KeyCode::Char('j') | KeyCode::Down => self.next_item(),
                        KeyCode::Char('k') | KeyCode::Up => self.previous_item(),
                        KeyCode::Char('x') if key.modifiers.is_empty() => {
                            self.toggle_complete().await?
                        }
                        KeyCode::Char('d') if key.modifiers.is_empty() => {
                            if self.is_browsing_projects() {
                                self.delete_project().await?
                            } else {
                                self.delete_task().await?
                            }
                        }
                        KeyCode::Char('c') | KeyCode::Char('C') => {
                            self.clear_completed().await?
                        }
                        KeyCode::Char('s') | KeyCode::Char('S') => {
                            self.sync().await?;
                        }
                        KeyCode::Char('1') => {
                            self.current_view = View::Inbox;
                            self.selected_index = 0;
                            self.selected_project_id = None;
                            self.previous_view = None;
                        }
                        KeyCode::Char('2') => {
                            self.current_view = View::Today;
                            self.selected_index = 0;
                            self.selected_project_id = None;
                            self.previous_view = None;
                        }
                        KeyCode::Char('3') => {
                            self.current_view = View::Upcoming;
                            self.selected_index = 0;
                            self.selected_project_id = None;
                            self.previous_view = None;
                        }
                        KeyCode::Char('4') => {
                            self.current_view = View::Projects;
                            self.selected_index = 0;
                            self.selected_project_id = None;
                            self.previous_view = None;
                        }
                        KeyCode::Char('5') => {
                            self.current_view = View::Completed;
                            self.selected_index = 0;
                            self.selected_project_id = None;
                            self.previous_view = None;
                        }
                        KeyCode::Char('6') => {
                            self.current_view = View::Recurring;
                            self.selected_index = 0;
                            self.selected_project_id = None;
                            self.previous_view = None;
                        }
                        KeyCode::Enter if self.current_view == View::Projects => {
                            if self.is_browsing_projects() {
                                // Select project to filter by it
                                if self.project_selected_index < self.project_counts.len() {
                                    let (proj_id, proj_name, _, _) = &self.project_counts[self.project_selected_index];
                                    self.selected_project_id = Some(*proj_id);
                                    self.selected_index = 0;
                                    self.status_message = Some(format!("Project: {}", proj_name));
                                }
                            } else {
                                // Deselect, back to project list
                                self.selected_project_id = None;
                                self.status_message = Some("Projects".to_string());
                            }
                        }
                        KeyCode::Esc => {
                            if self.selected_project_id.is_some() {
                                self.selected_project_id = None;
                                self.status_message = Some("Cleared project filter".to_string());
                            }
                        }
                        _ => {}
                    }
                }
                InputMode::Editing => match key.code {
                    KeyCode::Esc => {
                        self.input_mode = InputMode::Normal;
                        self.input_buffer.clear();
                        self.cursor_position = 0;
                    }
                    KeyCode::Enter => {
                        if self.current_view == View::Projects {
                            let name = self.input_buffer.trim().to_string();
                            if !name.is_empty() {
                                let project = Project::new(self.user_id, name.clone());
                                self.project_store.create(&project).await?;

                                // Record project creation operation for sync
                                let seq = self.op_store.get_next_seq(self.user_id, self.device_id).await?;
                                let op = Operation {
                                    op_id: Uuid::new_v4(),
                                    user_id: self.user_id,
                                    device_id: self.device_id,
                                    seq,
                                    entity: Entity::Project,
                                    entity_id: project.id,
                                    op_type: OperationType::Create,
                                    payload: OperationPayload::ProjectCreate {
                                        name: name.clone(),
                                        color: None,
                                        sort_order: 0,
                                    },
                                    created_at: chrono::Utc::now(),
                                    synced_at: None,
                                };
                                self.op_store.append(&op).await?;

                                self.status_message = Some(format!("Created project: {}", name));
                                self.refresh_tasks().await?;
                                self.request_sync_after_mutation();
                            }
                        } else {
                            self.create_task_from_input().await?;
                        }
                        self.input_buffer.clear();
                        self.cursor_position = 0;
                        self.input_mode = InputMode::Normal;
                    }
                    // Ctrl+A/E/W must come before catch-all Char(c)
                    KeyCode::Char('a') if key.modifiers == KeyModifiers::CONTROL => {
                        self.cursor_position = 0;
                    }
                    KeyCode::Char('e') if key.modifiers == KeyModifiers::CONTROL => {
                        self.cursor_position = self.input_buffer.len();
                    }
                    KeyCode::Char('w') if key.modifiers == KeyModifiers::CONTROL => {
                        let end = self.cursor_position;
                        let start = self.input_buffer[..end]
                            .rfind(' ')
                            .map(|i| i + 1)
                            .unwrap_or(0);
                        self.input_buffer.drain(start..end);
                        self.cursor_position = start;
                    }
                    KeyCode::Char(c) => {
                        self.input_buffer.insert(self.cursor_position, c);
                        self.cursor_position += c.len_utf8();
                    }
                    KeyCode::Backspace => {
                        if self.cursor_position > 0 {
                            let prev = self.input_buffer[..self.cursor_position].chars().last().unwrap();
                            self.cursor_position -= prev.len_utf8();
                            self.input_buffer.remove(self.cursor_position);
                        }
                    }
                    KeyCode::Left => {
                        if self.cursor_position > 0 {
                            let prev = self.input_buffer[..self.cursor_position].chars().last().unwrap();
                            self.cursor_position -= prev.len_utf8();
                        }
                    }
                    KeyCode::Right => {
                        if self.cursor_position < self.input_buffer.len() {
                            let next = self.input_buffer[self.cursor_position..].chars().next().unwrap();
                            self.cursor_position += next.len_utf8();
                        }
                    }
                    KeyCode::Home => {
                        self.cursor_position = 0;
                    }
                    KeyCode::End => {
                        self.cursor_position = self.input_buffer.len();
                    }
                    _ => {}
                },
                InputMode::EditingTask(task_id) => match key.code {
                    KeyCode::Esc => {
                        self.input_mode = InputMode::Normal;
                        self.input_buffer.clear();
                    }
                    KeyCode::Enter => {
                        self.update_task_from_input(task_id).await?;
                        self.input_buffer.clear();
                        self.input_mode = InputMode::Normal;
                    }
                    // Ctrl+A/E must come before catch-all Char(c)
                    KeyCode::Char('a') if key.modifiers == KeyModifiers::CONTROL => {
                        self.cursor_position = 0;
                    }
                    KeyCode::Char('e') if key.modifiers == KeyModifiers::CONTROL => {
                        self.cursor_position = self.input_buffer.len();
                    }
                    KeyCode::Char(c) => {
                        self.input_buffer.insert(self.cursor_position, c);
                        self.cursor_position += c.len_utf8();
                    }
                    KeyCode::Backspace => {
                        if self.cursor_position > 0 {
                            let prev = self.input_buffer[..self.cursor_position].chars().last().unwrap();
                            self.cursor_position -= prev.len_utf8();
                            self.input_buffer.remove(self.cursor_position);
                        }
                    }
                    KeyCode::Left => {
                        if self.cursor_position > 0 {
                            let prev = self.input_buffer[..self.cursor_position].chars().last().unwrap();
                            self.cursor_position -= prev.len_utf8();
                        }
                    }
                    KeyCode::Right => {
                        if self.cursor_position < self.input_buffer.len() {
                            let next = self.input_buffer[self.cursor_position..].chars().next().unwrap();
                            self.cursor_position += next.len_utf8();
                        }
                    }
                    KeyCode::Home => {
                        self.cursor_position = 0;
                    }
                    KeyCode::End => {
                        self.cursor_position = self.input_buffer.len();
                    }
                    _ => {}
                },
                InputMode::Searching => match key.code {
                    KeyCode::Esc => {
                        self.input_mode = InputMode::Normal;
                        self.search_query.clear();
                        if let Some(view) = self.previous_view.take() {
                            self.current_view = view;
                        }
                        self.clamp_selection();
                    }
                    KeyCode::Enter => {
                        self.input_mode = InputMode::Normal;
                        self.clamp_selection();
                    }
                    // Ctrl+A/E must come before catch-all Char(c)
                    KeyCode::Char('a') if key.modifiers == KeyModifiers::CONTROL => {
                        self.cursor_position = 0;
                    }
                    KeyCode::Char('e') if key.modifiers == KeyModifiers::CONTROL => {
                        self.cursor_position = self.search_query.len();
                    }
                    KeyCode::Char(c) => {
                        self.search_query.insert(self.cursor_position, c);
                        self.cursor_position += c.len_utf8();
                        self.clamp_selection();
                    }
                    KeyCode::Backspace => {
                        if self.cursor_position > 0 {
                            let prev = self.search_query[..self.cursor_position].chars().last().unwrap();
                            self.cursor_position -= prev.len_utf8();
                            self.search_query.remove(self.cursor_position);
                            self.clamp_selection();
                        }
                    }
                    KeyCode::Left => {
                        if self.cursor_position > 0 {
                            let prev = self.search_query[..self.cursor_position].chars().last().unwrap();
                            self.cursor_position -= prev.len_utf8();
                        }
                    }
                    KeyCode::Right => {
                        if self.cursor_position < self.search_query.len() {
                            let next = self.search_query[self.cursor_position..].chars().next().unwrap();
                            self.cursor_position += next.len_utf8();
                        }
                    }
                    KeyCode::Home => {
                        self.cursor_position = 0;
                    }
                    KeyCode::End => {
                        self.cursor_position = self.search_query.len();
                    }
                    _ => {}
                },
            }
        }
        Ok(())
    }

    /// Re-parse the input buffer and update all task properties.
    async fn update_task_from_input(&mut self, task_id: Uuid) -> Result<()> {
        let input = self.input_buffer.trim().to_string();
        if input.is_empty() {
            return Ok(());
        }

        // Find task by ID
        let task_idx = self.tasks.iter().position(|t| t.id == task_id);
        let task_idx = match task_idx {
            Some(i) => i,
            None => return Ok(()),
        };

        let old_title = self.tasks[task_idx].title.clone();
        let old_project_id = self.tasks[task_idx].project_id;

        // Re-parse input to get all fields
        let parsed = NaturalLanguageParser::parse(&input);
        let due_at = parsed.resolve_datetime();
        let priority = parsed.priority;
        let new_title = parsed.title;

        if new_title.is_empty() {
            return Ok(());
        }

        let task = &mut self.tasks[task_idx];
        task.title = new_title.clone();
        task.priority = priority.clone();
        task.due_at = due_at;
        task.updated_at = chrono::Utc::now();

        // Handle project
        if let Some(ref project_name) = parsed.project {
            let existing = self.project_store.find_by_name(self.user_id, project_name).await?;
            task.project_id = Some(match existing {
                Some(p) => p.id,
                None => {
                    let new_project = Project::new(self.user_id, project_name.clone());
                    self.project_store.create(&new_project).await?;
                    let seq = self.op_store.get_next_seq(self.user_id, self.device_id).await?;
                    let op = Operation {
                        op_id: Uuid::new_v4(),
                        user_id: self.user_id,
                        device_id: self.device_id,
                        seq,
                        entity: todomrs_sync::operations::Entity::Project,
                        entity_id: new_project.id,
                        op_type: todomrs_sync::operations::OperationType::Create,
                        payload: todomrs_sync::operations::OperationPayload::ProjectCreate {
                            name: project_name.clone(),
                            color: None,
                            sort_order: 0,
                        },
                        created_at: chrono::Utc::now(),
                        synced_at: None,
                    };
                    self.op_store.append(&op).await?;
                    new_project.id
                }
            });
        } else {
            task.project_id = None;
        }

        // Handle recurrence changes on edit
        let old_recurrence_rule_id = task.recurrence_rule_id;
        match (&parsed.recurrence, task.recurrence_rule_id) {
            (Some(_rec), None) => {
                // New recurrence — create rule
                let (_, rule_opt) = NaturalLanguageParser::create_task_from_input(&input, self.user_id);
                if let Some(mut rule) = rule_opt {
                    // Fix: parser creates rule with throwaway task_id — use actual task
                    rule.task_id = task_id;
                    self.recurrence_store.create(&rule).await?;
                    task.recurrence_rule_id = Some(rule.id);

                    let seq = self.op_store.get_next_seq(self.user_id, self.device_id).await?;
                    let op_op = Operation::create_recurrence_rule(self.user_id, self.device_id, seq, &rule);
                    self.op_store.append(&op_op).await?;
                }
            }
            (Some(_rec), Some(rule_id)) => {
                // Changed recurrence — update existing rule
                if let Some(mut rule) = self.recurrence_rules.get(&rule_id).cloned() {
                    let (_, new_rule_opt) = NaturalLanguageParser::create_task_from_input(&input, self.user_id);
                    if let Some(new_rule) = new_rule_opt {
                        // Only update if something actually changed
                        if rule.interval != new_rule.interval
                            || rule.kind != new_rule.kind
                            || rule.wait_for_completion != new_rule.wait_for_completion
                            || rule.anchor_mode != new_rule.anchor_mode
                        {
                            rule.interval = new_rule.interval;
                            rule.kind = new_rule.kind;
                            rule.wait_for_completion = new_rule.wait_for_completion;
                            rule.anchor_mode = new_rule.anchor_mode;
                            rule.updated_at = chrono::Utc::now();
                            self.recurrence_store.update(&rule).await?;

                            // Sync: record rule update operation
                            let seq = self.op_store.get_next_seq(self.user_id, self.device_id).await?;
                            let rule_op = Operation {
                                op_id: Uuid::new_v4(),
                                user_id: self.user_id,
                                device_id: self.device_id,
                                seq,
                                entity: todomrs_sync::operations::Entity::RecurrenceRule,
                                entity_id: rule.id,
                                op_type: todomrs_sync::operations::OperationType::Update,
                                payload: todomrs_sync::operations::OperationPayload::RecurrenceRuleUpdate {
                                    interval: Some(rule.interval),
                                    wait_for_completion: Some(rule.wait_for_completion),
                                    anchor_mode: Some(serde_json::to_value(&rule.anchor_mode).ok().and_then(|v| v.as_str().map(|s| s.to_string())).unwrap_or_default()),
                                },
                                created_at: chrono::Utc::now(),
                                synced_at: None,
                            };
                            self.op_store.append(&rule_op).await?;
                        }
                    }
                }
            }
            (None, Some(rule_id)) => {
                // Removed recurrence — delete rule
                self.recurrence_store.delete(rule_id).await?;
                task.recurrence_rule_id = None;

                // Sync: record rule delete operation
                let seq = self.op_store.get_next_seq(self.user_id, self.device_id).await?;
                let del_op = Operation {
                    op_id: Uuid::new_v4(),
                    user_id: self.user_id,
                    device_id: self.device_id,
                    seq,
                    entity: todomrs_sync::operations::Entity::RecurrenceRule,
                    entity_id: rule_id,
                    op_type: todomrs_sync::operations::OperationType::Delete,
                    payload: todomrs_sync::operations::OperationPayload::Delete,
                    created_at: chrono::Utc::now(),
                    synced_at: None,
                };
                self.op_store.append(&del_op).await?;
            }
            (None, None) => {} // No change
        }

        self.task_store.update(task).await?;

        // Record update operation
        let seq = self.op_store.get_next_seq(self.user_id, self.device_id).await?;
        let op = Operation {
            op_id: Uuid::new_v4(),
            user_id: self.user_id,
            device_id: self.device_id,
            seq,
            entity: todomrs_sync::operations::Entity::Task,
            entity_id: task_id,
            op_type: todomrs_sync::operations::OperationType::Update,
            payload: todomrs_sync::operations::OperationPayload::TaskUpdate {
                title: Some(new_title.clone()),
                description: None,
                status: None,
                project_id: task.project_id,
                tag_ids: None,
                priority: Some(priority),
                due_at: due_at,
                scheduled_at: None,
                recurrence_rule_id: task.recurrence_rule_id,
                completed_at: None,
            },
            created_at: chrono::Utc::now(),
            synced_at: None,
        };
        self.op_store.append(&op).await?;

        let changed_parts = {
            let mut parts = Vec::new();
            if new_title != old_title {
                parts.push(format!("title"));
            }
            if task.project_id != old_project_id {
                parts.push(format!("project"));
            }
            if task.recurrence_rule_id != old_recurrence_rule_id {
                parts.push(format!("recurrence"));
            }
            parts.join(", ")
        };

        self.status_message = Some(format!("Updated {}", changed_parts));
        self.refresh_tasks().await?;
        self.request_sync_after_mutation();
        Ok(())
    }

    /// Parse the input buffer, create a task and operation, persist both.
    async fn create_task_from_input(&mut self) -> Result<()> {
        let input = self.input_buffer.trim().to_string();
        if input.is_empty() {
            return Ok(());
        }

        let (mut task, recurrence_rule) =
            NaturalLanguageParser::create_task_from_input(&input, self.user_id);

        // Handle +project: look up or create project
        let parsed = NaturalLanguageParser::parse(&input);
        if let Some(ref project_name) = parsed.project {
            let existing = self.project_store.find_by_name(self.user_id, project_name).await?;
            let project_id = match existing {
                Some(p) => p.id,
                None => {
                    let new_project = Project::new(self.user_id, project_name.clone());
                    self.project_store.create(&new_project).await?;

                    // Record project creation operation
                    let seq = self.op_store.get_next_seq(self.user_id, self.device_id).await?;
                    let op = Operation {
                        op_id: Uuid::new_v4(),
                        user_id: self.user_id,
                        device_id: self.device_id,
                        seq,
                        entity: todomrs_sync::operations::Entity::Project,
                        entity_id: new_project.id,
                        op_type: todomrs_sync::operations::OperationType::Create,
                        payload: todomrs_sync::operations::OperationPayload::ProjectCreate {
                            name: project_name.clone(),
                            color: None,
                            sort_order: 0,
                        },
                        created_at: chrono::Utc::now(),
                        synced_at: None,
                    };
                    self.op_store.append(&op).await?;

                    new_project.id
                }
            };
            task.project_id = Some(project_id);
        }

        // Persist task FIRST (rule FK depends on task existing)
        self.task_store.create(&task).await?;

        // Persist recurrence rule if present (AFTER task, due to FK constraint)
        if let Some(rule) = &recurrence_rule {
            self.recurrence_store.create(rule).await?;
            let seq = self.op_store.get_next_seq(self.user_id, self.device_id).await?;
            let op = Operation::create_recurrence_rule(self.user_id, self.device_id, seq, rule);
            self.op_store.append(&op).await?;
        }

        // Record task operation for sync
        let seq = self.op_store.get_next_seq(self.user_id, self.device_id).await?;
        let op = Operation::create_task(self.user_id, self.device_id, seq, &task);
        self.op_store.append(&op).await?;

        self.status_message = Some(format!("Created: {}", task.title));
        self.refresh_tasks().await?;
        self.request_sync_after_mutation();
        Ok(())
    }

    /// Toggle the selected task between completed and pending.
    async fn toggle_complete(&mut self) -> Result<()> {
        let filtered = self.filtered_tasks();
        if self.selected_index >= filtered.len() {
            return Ok(());
        }

        let mut task = filtered[self.selected_index].clone();
        let completed = task.status == TaskStatus::Completed;
        let (description, op) = if completed {
            task.uncomplete();
            let seq = self.op_store.get_next_seq(self.user_id, self.device_id).await?;
            let op = Operation {
                op_id: Uuid::new_v4(),
                user_id: self.user_id,
                device_id: self.device_id,
                seq,
                entity: todomrs_sync::operations::Entity::Task,
                entity_id: task.id,
                op_type: todomrs_sync::operations::OperationType::Update,
                payload: todomrs_sync::operations::OperationPayload::TaskUpdate {
                    title: None,
                    description: None,
                    status: Some(TaskStatus::Pending),
                    project_id: None,
                    tag_ids: None,
                    priority: None,
                    due_at: None,
                    scheduled_at: None,
                    recurrence_rule_id: None,
                    completed_at: None,
                },
                created_at: chrono::Utc::now(),
                synced_at: None,
            };
            ("Uncompleted", op)
        } else {
            task.complete();
            let seq = self.op_store.get_next_seq(self.user_id, self.device_id).await?;
            let op = Operation::complete_task(self.user_id, self.device_id, seq, task.id);
            ("Completed", op)
        };

        task.updated_at = chrono::Utc::now();
        self.task_store.update(&task).await?;
        self.op_store.append(&op).await?;

        // When completing a recurring task, spawn the next instance
        if !completed {
            if let Some(rule_id) = task.recurrence_rule_id {
                if let Some(rule) = self.recurrence_rules.get(&rule_id).cloned() {
                    self.spawn_next_recurrence(&task, &rule).await?;
                    return Ok(()); // refresh handled by spawn_next
                }
            }
        }

        self.status_message = Some(format!("{}: {}", description, task.title));
        self.refresh_tasks().await?;
        self.request_sync_after_mutation();
        Ok(())
    }

    /// Soft-delete the selected task.
    async fn delete_task(&mut self) -> Result<()> {
        let filtered = self.filtered_tasks();
        if self.selected_index >= filtered.len() {
            return Ok(());
        }

        let task = filtered[self.selected_index].clone();
        let title = task.title.clone();

        self.task_store.soft_delete(task.id).await?;

        // Record delete operation
        let seq = self.op_store.get_next_seq(self.user_id, self.device_id).await?;
        let op = Operation {
            op_id: Uuid::new_v4(),
            user_id: self.user_id,
            device_id: self.device_id,
            seq,
            entity: todomrs_sync::operations::Entity::Task,
            entity_id: task.id,
            op_type: todomrs_sync::operations::OperationType::Delete,
            payload: todomrs_sync::operations::OperationPayload::Delete,
            created_at: chrono::Utc::now(),
            synced_at: None,
        };
        self.op_store.append(&op).await?;

        // Adjust selection
        if self.selected_index > 0 {
            self.selected_index -= 1;
        }

        self.status_message = Some(format!("Deleted: {}", title));
        self.refresh_tasks().await?;
        self.request_sync_after_mutation();
        Ok(())
    }

    /// Soft-delete all completed tasks.
    async fn clear_completed(&mut self) -> Result<()> {
        let completed: Vec<Task> = self
            .tasks
            .iter()
            .filter(|t| t.status == TaskStatus::Completed && t.deleted_at.is_none())
            .cloned()
            .collect();

        let count = completed.len();
        if count == 0 {
            self.status_message = Some("No completed tasks to clear".to_string());
            return Ok(());
        }

        for task in completed {
            self.task_store.soft_delete(task.id).await?;

            let seq = self.op_store.get_next_seq(self.user_id, self.device_id).await?;
            let op = Operation {
                op_id: Uuid::new_v4(),
                user_id: self.user_id,
                device_id: self.device_id,
                seq,
                entity: todomrs_sync::operations::Entity::Task,
                entity_id: task.id,
                op_type: todomrs_sync::operations::OperationType::Delete,
                payload: todomrs_sync::operations::OperationPayload::Delete,
                created_at: chrono::Utc::now(),
                synced_at: None,
            };
            self.op_store.append(&op).await?;
        }

        self.status_message = Some(format!("Cleared {} completed tasks", count));
        self.selected_index = 0;
        self.refresh_tasks().await?;
        self.request_sync_after_mutation();
        Ok(())
    }

    /// Create the next instance of a recurring task after completion.
    async fn spawn_next_recurrence(&mut self, completed_task: &Task, rule: &RecurrenceRule) -> Result<()> {
        // Determine anchor date for next occurrence
        let anchor = match rule.anchor_mode {
            AnchorMode::Schedule => completed_task.due_at.unwrap_or(completed_task.created_at),
            AnchorMode::Completion => completed_task.completed_at.unwrap_or_else(chrono::Utc::now),
        };

        // Compute next due date
        let next_due = RecurrenceEngine::next_occurrence(rule, anchor);

        // Create new task instance (independent copy)
        let mut new_task = Task::new(self.user_id, completed_task.title.clone());
        new_task.project_id = completed_task.project_id;
        new_task.tag_ids = completed_task.tag_ids.clone();
        new_task.priority = completed_task.priority.clone();
        new_task.due_at = Some(next_due);
        new_task.recurrence_rule_id = Some(rule.id); // links back to the same rule

        // Persist
        self.task_store.create(&new_task).await?;

        // Record operation
        let seq = self.op_store.get_next_seq(self.user_id, self.device_id).await?;
        let op = Operation::create_task(self.user_id, self.device_id, seq, &new_task);
        self.op_store.append(&op).await?;

        self.status_message = Some(format!(
            "Completed: {} | Next: {}",
            completed_task.title,
            format_recurrence_rule(rule)
        ));

        self.refresh_tasks().await?;
        self.request_sync_after_mutation();
        Ok(())
    }

    /// Check for debounced or periodic auto-sync.
    /// Returns true if a sync was triggered.
    pub async fn maybe_auto_sync(&mut self) -> bool {
        let should_periodic = self.last_sync_attempt.elapsed() >= std::time::Duration::from_secs(30);
        // Debounce: 10s since the LAST MUTATION, not since last sync.
        let should_debounce = self.sync_debounce_requested
            && self.last_mutation_at.elapsed() >= std::time::Duration::from_secs(10);

        if (should_periodic || should_debounce) && self.sync_client.is_some() {
            self.sync_debounce_requested = false;
            self.sync().await.ok();
            return true;
        }
        false
    }

    /// Mark that a mutation has occurred — sync will fire 10s after the last one.
    pub fn request_sync_after_mutation(&mut self) {
        self.last_mutation_at = Instant::now();
        self.sync_debounce_requested = true;
    }

    /// Perform a full sync cycle: upload local ops, download remote ops, apply them.
    pub async fn sync(&mut self) -> Result<()> {
        let client = match &self.sync_client {
            Some(c) if c.is_authenticated() => c,
            _ => {
                self.sync_status = SyncStatus::Disabled;
                return Ok(());
            }
        };

        self.sync_status = SyncStatus::Syncing;
        self.last_sync_attempt = Instant::now();

        // 1. Upload local unsynced operations
        let upload_count = match self.op_store.get_unsynced(self.user_id).await {
            Ok(unsynced) if !unsynced.is_empty() => {
                let count = unsynced.len();
                if let Err(e) = client.upload_operations(unsynced.clone()).await {
                    self.sync_status = SyncStatus::Offline(format!("Upload failed: {}", e));
                    self.status_message = Some(format!("Sync upload failed: {}", e));
                    return Ok(());
                }
                let op_ids: Vec<Uuid> = unsynced.iter().map(|op| op.op_id).collect();
                self.op_store.mark_synced(&op_ids).await?;
                count
            }
            Ok(_) => 0,
            Err(e) => {
                self.sync_status = SyncStatus::Offline(format!("DB error: {}", e));
                self.status_message = Some(format!("Sync DB error: {}", e));
                return Ok(());
            }
        };

        // 2. Download remote operations created after last_synced_at
        let remote_ops = match client.get_operations(self.last_synced_at).await {
            Ok(ops) => ops,
            Err(e) => {
                self.sync_status = SyncStatus::Offline(format!("Download failed: {}", e));
                self.status_message = Some(format!("Sync download failed: {}", e));
                return Ok(());
            }
        };

        // 3. Apply remote operations (skip our own)
        //    Use the Supabase auth user ID for the check, since operations from
        //    other clients (e.g. PWA) will have the Supabase user ID. Fall back
        //    to the local user_id if sync client doesn't have the supabase ID yet.
        let expected_user_id = self.sync_client
            .as_ref()
            .and_then(|c| c.supabase_user_id())
            .unwrap_or(self.user_id);
        let mut newest_time = self.last_synced_at;
        let mut applied_count = 0;
        for op in &remote_ops {
            if op.created_at > newest_time {
                newest_time = op.created_at;
            }
            if op.device_id == self.device_id || op.user_id != expected_user_id {
                continue;
            }
            if let Err(e) = self.apply_remote_operation(op).await {
                eprintln!("Failed to apply remote op {:?}: {}", op.op_id, e);
            } else {
                applied_count += 1;
            }
        }
        self.last_synced_at = newest_time;

        // 4. Refresh UI
        self.refresh_tasks().await?;

        self.sync_status = SyncStatus::Synced;

        // Build a status message showing what sync did
        let status_msg = if upload_count > 0 && applied_count > 0 {
            format!("Synced (↑{} uploaded, ↓{} applied)", upload_count, applied_count)
        } else if upload_count > 0 {
            format!("Synced ({} uploaded)", upload_count)
        } else if applied_count > 0 {
            format!("Synced ({} remote ops)", applied_count)
        } else {
            "Synced".to_string()
        };
        self.status_message = Some(status_msg);

        // Check notifications after successful sync
        if let Err(e) = notifications::check_notifications(self.task_store.pool(), &self.tasks).await {
            eprintln!("Notification check failed: {}", e);
        }

        Ok(())
    }

    /// Apply a single remote operation to the local database.
    ///
    /// Uses idempotent checks to avoid duplicates. Skips operations
    /// that reference non-existent FK targets rather than crashing.
    async fn apply_remote_operation(&mut self, op: &Operation) -> Result<()> {
        match (&op.entity, &op.op_type) {
            // ── Task Create ───────────────────────────────────────────
            (Entity::Task, OperationType::Create) => {
                if let OperationPayload::TaskCreate {
                    title, description, status, project_id, tag_ids: _,
                    priority, due_at, scheduled_at, recurrence_rule_id,
                } = &op.payload {
                    // Skip if already exists (idempotent)
                    if self.task_store.get_by_id(op.entity_id).await?.is_some() {
                        return Ok(());
                    }

                    // Guard: skip tag_ids that don't exist locally (FK constraint)
                    // We skip all tags on remote tasks since we can't easily verify
                    // tag existence without a TagStore lookup per tag.
                    let valid_tag_ids: Vec<Uuid> = Vec::new();

                    let mut task = Task::new(self.user_id, title.clone());
                    task.id = op.entity_id;
                    task.description = description.clone();
                    task.status = status.clone();
                    task.tag_ids = valid_tag_ids;
                    task.priority = priority.clone();
                    task.due_at = *due_at;
                    task.scheduled_at = *scheduled_at;
                    task.created_at = op.created_at;
                    task.updated_at = op.created_at;

                    // Only set project_id if the project exists locally
                    if let Some(pid) = project_id {
                        if self.project_store.get_by_id(*pid).await?.is_some() {
                            task.project_id = Some(*pid);
                        }
                        // else: project doesn't exist yet — leave unassigned
                    }

                    // Only set recurrence_rule_id if the rule exists locally
                    if let Some(rid) = recurrence_rule_id {
                        if self.recurrence_store.get_by_id(*rid).await?.is_some() {
                            task.recurrence_rule_id = Some(*rid);
                        }
                        // else: rule doesn't exist yet — leave unassigned
                    }

                    // Create without tag_ids (tag FK validation not available)
                    self.task_store.create(&task).await?;
                }
            }

            // ── Task Update ───────────────────────────────────────────
            (Entity::Task, OperationType::Update) => {
                if let Some(mut task) = self.task_store.get_by_id(op.entity_id).await? {
                    if let OperationPayload::TaskUpdate {
                        title, description, status, project_id, tag_ids: _,
                        priority, due_at, scheduled_at, recurrence_rule_id,
                        completed_at,
                    } = &op.payload {
                        if let Some(t) = title { task.title = t.clone(); }
                        if let Some(d) = description { task.description = Some(d.clone()); }
                        if let Some(s) = status { task.status = s.clone(); }
                        if let Some(p) = project_id { task.project_id = Some(*p); }
                        if let Some(p) = priority { task.priority = p.clone(); }
                        if let Some(d) = due_at { task.due_at = Some(*d); }
                        if let Some(d) = scheduled_at { task.scheduled_at = Some(*d); }
                        if let Some(r) = recurrence_rule_id { task.recurrence_rule_id = Some(*r); }
                        if let Some(c) = completed_at { task.completed_at = Some(*c); }
                        task.updated_at = op.created_at;
                        self.task_store.update(&task).await?;
                    }
                }
                // Task doesn't exist locally — skip (may have been deleted on this device)
            }

            // ── Task Delete ───────────────────────────────────────────
            (Entity::Task, OperationType::Delete) => {
                self.task_store.soft_delete(op.entity_id).await?;
            }

            // ── Project Create ────────────────────────────────────────
            (Entity::Project, OperationType::Create) => {
                if let OperationPayload::ProjectCreate { name, color, sort_order } = &op.payload {
                    if self.project_store.get_by_id(op.entity_id).await?.is_some() {
                        return Ok(());
                    }
                    let project = Project {
                        id: op.entity_id,
                        user_id: self.user_id,
                        name: name.clone(),
                        color: color.clone(),
                        sort_order: *sort_order,
                        created_at: op.created_at,
                        updated_at: op.created_at,
                        archived_at: None,
                    };
                    self.project_store.create(&project).await?;
                }
            }

            // ── Project Update ────────────────────────────────────────
            (Entity::Project, OperationType::Update) => {
                if let Some(mut project) = self.project_store.get_by_id(op.entity_id).await? {
                    if let OperationPayload::ProjectUpdate { name, color, sort_order, archived_at } = &op.payload {
                        if let Some(n) = name { project.name = n.clone(); }
                        if let Some(c) = color { project.color = Some(c.clone()); }
                        if let Some(s) = sort_order { project.sort_order = *s; }
                        if let Some(a) = archived_at { project.archived_at = Some(*a); }
                        project.updated_at = op.created_at;
                        self.project_store.update(&project).await?;
                    }
                }
            }

            // ── Project Delete ────────────────────────────────────────
            (Entity::Project, OperationType::Delete) => {
                self.project_store.soft_delete(op.entity_id).await?;
            }

            // ── RecurrenceRule Create ─────────────────────────────────
            (Entity::RecurrenceRule, OperationType::Create) => {
                if let OperationPayload::RecurrenceRuleCreate {
                    task_id, kind, interval, timezone,
                    wait_for_completion, anchor_mode,
                } = &op.payload {
                    if self.recurrence_store.get_by_id(op.entity_id).await?.is_some() {
                        return Ok(());
                    }
                    // Guard: only create if the referenced task exists locally
                    if self.task_store.get_by_id(*task_id).await?.is_none() {
                        return Ok(());
                    }
                    let now = op.created_at;
                    let rule = todomrs_core::domain::RecurrenceRule {
                        id: op.entity_id,
                        task_id: *task_id,
                        kind: deserialize_recurrence_kind(kind),
                        interval: *interval,
                        by_weekday: None,
                        by_monthday: None,
                        timezone: timezone.clone(),
                        wait_for_completion: *wait_for_completion,
                        anchor_mode: deserialize_anchor_mode(anchor_mode),
                        created_at: now,
                        updated_at: now,
                    };
                    self.recurrence_store.create(&rule).await?;
                }
            }

            // ── RecurrenceRule Update ─────────────────────────────────
            (Entity::RecurrenceRule, OperationType::Update) => {
                if let Some(mut rule) = self.recurrence_store.get_by_id(op.entity_id).await? {
                    if let OperationPayload::RecurrenceRuleUpdate {
                        interval, wait_for_completion, anchor_mode,
                    } = &op.payload {
                        if let Some(i) = interval { rule.interval = *i; }
                        if let Some(w) = wait_for_completion { rule.wait_for_completion = *w; }
                        if let Some(a) = anchor_mode { rule.anchor_mode = deserialize_anchor_mode(a); }
                        rule.updated_at = op.created_at;
                        self.recurrence_store.update(&rule).await?;
                    }
                }
            }

            // ── RecurrenceRule Delete ─────────────────────────────────
            (Entity::RecurrenceRule, OperationType::Delete) => {
                self.recurrence_store.delete(op.entity_id).await?;
            }

            // ── Generic Delete (fallback for Tag, Reminder, etc.) ─────
            (_, OperationType::Delete) => {
                self.task_store.soft_delete(op.entity_id).await.ok();
            }

            _ => {} // Tag operations, reminders — skip for now
        }

        Ok(())
    }

    /// Refresh project counts for sidebar display.
    async fn refresh_project_counts(&mut self) -> Result<()> {
        let projects = self.project_store.get_all(self.user_id).await?;
        let mut counts = Vec::new();

        for project in &projects {
            let pending = self
                .tasks
                .iter()
                .filter(|t| t.project_id == Some(project.id) && t.status == TaskStatus::Pending)
                .count();
            let completed = self
                .tasks
                .iter()
                .filter(|t| t.project_id == Some(project.id) && t.status == TaskStatus::Completed)
                .count();
            counts.push((project.id, project.name.clone(), pending, completed));
        }

        self.project_counts = counts;
        Ok(())
    }

    pub fn next_item(&mut self) {
        if self.is_browsing_projects() {
            if self.project_selected_index + 1 < self.project_counts.len() {
                self.project_selected_index += 1;
            }
        } else {
            let count = self.filtered_tasks().len();
            if count > 0 && self.selected_index + 1 < count {
                self.selected_index += 1;
            }
        }
    }

    pub fn previous_item(&mut self) {
        if self.is_browsing_projects() {
            if self.project_selected_index > 0 {
                self.project_selected_index -= 1;
            }
        } else if self.selected_index > 0 {
            self.selected_index -= 1;
        }
    }

    fn clamp_selection(&mut self) {
        if self.is_browsing_projects() {
            if self.project_counts.is_empty() {
                self.project_selected_index = 0;
            } else if self.project_selected_index >= self.project_counts.len() {
                self.project_selected_index = self.project_counts.len().saturating_sub(1);
            }
        } else {
            let count = self.filtered_tasks().len();
            if count > 0 && self.selected_index >= count {
                self.selected_index = count.saturating_sub(1);
            }
        }
    }

    /// Reconstruct a natural-language edit string from a task's properties.
    fn task_to_edit_string(&self, task: &Task) -> String {
        let mut parts = Vec::new();

        // Title
        parts.push(task.title.clone());

        // Project
        if let Some(project_id) = task.project_id {
            if let Some((_, name, _, _)) = self.project_counts.iter().find(|(id, _, _, _)| *id == project_id) {
                parts.push(format!("+{}", name));
            }
        }

        // Priority
        let p_str = match task.priority {
            Priority::Urgent => "p1",
            Priority::High => "p2",
            Priority::Medium => "p3",
            Priority::Low => "p4",
            Priority::None => "",
        };
        if !p_str.is_empty() {
            parts.push(p_str.to_string());
        }

        // Due date/time
        if let Some(dt) = task.due_at {
            parts.push(format_datetime_for_edit(dt));
        }

        // Recurrence pattern
        if let Some(rule_id) = task.recurrence_rule_id {
            if let Some(rule) = self.recurrence_rules.get(&rule_id) {
                // Prefix for wait_for_completion
                if rule.wait_for_completion {
                    parts.push("wait!".to_string());
                }

                // Prefix for anchor_mode
                let every_prefix = match rule.anchor_mode {
                    AnchorMode::Completion => "every!",
                    AnchorMode::Schedule => "every",
                };

                let kind_str = match rule.kind {
                    todomrs_core::domain::RecurrenceKind::Daily => "day",
                    todomrs_core::domain::RecurrenceKind::Weekly => "week",
                    todomrs_core::domain::RecurrenceKind::Monthly => "month",
                    todomrs_core::domain::RecurrenceKind::Yearly => "year",
                };

                parts.push(if rule.interval == 1 {
                    format!("{} {}", every_prefix, kind_str)
                } else {
                    format!("{} {} {}", every_prefix, rule.interval, kind_str)
                });
            }
        }

        parts.join(" ")
    }

    /// Whether we're browsing the project list (not filtering by one).
    pub fn is_browsing_projects(&self) -> bool {
        self.current_view == View::Projects && self.selected_project_id.is_none()
    }

    /// Add a project from the Projects view.
    async fn add_project(&mut self) -> Result<()> {
        self.input_buffer.clear();
        self.cursor_position = 0;
        self.input_mode = InputMode::Editing;
        Ok(())
    }

    /// Delete the selected project.
    async fn delete_project(&mut self) -> Result<()> {
        if self.project_selected_index >= self.project_counts.len() {
            return Ok(());
        }

        let (proj_id, name, _, _) = &self.project_counts[self.project_selected_index];
        let proj_id = *proj_id;
        let name = name.clone();

        self.project_store.soft_delete(proj_id).await?;

        // Record project deletion operation for sync
        let seq = self.op_store.get_next_seq(self.user_id, self.device_id).await?;
        let op = Operation {
            op_id: Uuid::new_v4(),
            user_id: self.user_id,
            device_id: self.device_id,
            seq,
            entity: Entity::Project,
            entity_id: proj_id,
            op_type: OperationType::Delete,
            payload: OperationPayload::Delete,
            created_at: chrono::Utc::now(),
            synced_at: None,
        };
        self.op_store.append(&op).await?;

        // Unlink tasks that were assigned to this project
        for task in self.tasks.iter_mut().filter(|t| t.project_id == Some(proj_id)) {
            task.project_id = None;
            self.task_store.update(task).await?;
        }

        if self.project_selected_index > 0 && self.project_selected_index >= self.project_counts.len().saturating_sub(1) {
            self.project_selected_index -= 1;
        }

        self.status_message = Some(format!("Deleted project: {}", name));
        self.refresh_tasks().await?;
        self.request_sync_after_mutation();
        Ok(())
    }
}

/// Format a recurrence rule into a human-readable string.
pub fn format_recurrence_rule(rule: &RecurrenceRule) -> String {
    let kind_str = match rule.kind {
        todomrs_core::domain::RecurrenceKind::Daily => "day",
        todomrs_core::domain::RecurrenceKind::Weekly => "week",
        todomrs_core::domain::RecurrenceKind::Monthly => "month",
        todomrs_core::domain::RecurrenceKind::Yearly => "year",
    };

    let base = if rule.interval == 1 {
        format!("every {}", kind_str)
    } else {
        format!("every {} {}s", rule.interval, kind_str)
    };

    let prefix = match rule.wait_for_completion {
        true => "wait! ",
        false => "",
    };
    let suffix = match rule.anchor_mode {
        AnchorMode::Completion => " (from completion)",
        AnchorMode::Schedule => "",
    };

    format!("{}{}{}", prefix, base, suffix)
}

/// Format a DateTime for natural-language editing (parser-friendly).
/// Converts UTC to local time for display.
fn format_datetime_for_edit(dt: chrono::DateTime<chrono::Utc>) -> String {
    use chrono::Duration;

    let local_dt = dt.with_timezone(&chrono::Local);
    let today = chrono::Local::now().date_naive();
    let date = local_dt.date_naive();
    let time = local_dt.time();

    let date_part = if date == today {
        "today".to_string()
    } else if date == today + Duration::days(1) {
        "tomorrow".to_string()
    } else {
        match date.weekday() {
            Weekday::Mon => "monday",
            Weekday::Tue => "tuesday",
            Weekday::Wed => "wednesday",
            Weekday::Thu => "thursday",
            Weekday::Fri => "friday",
            Weekday::Sat => "saturday",
            Weekday::Sun => "sunday",
        }
        .to_string()
    };

    let midnight = NaiveTime::from_hms_opt(0, 0, 0).unwrap();
    if time != midnight {
        format!("{} {:02}:{:02}", date_part, time.hour(), time.minute())
    } else {
        date_part
    }
}

// ── Sync deserialization helpers ─────────────────────────────────────

/// Deserialize a recurrence kind string to its enum variant.
fn deserialize_recurrence_kind(s: &str) -> todomrs_core::domain::RecurrenceKind {
    match s.to_lowercase().as_str() {
        "daily" => todomrs_core::domain::RecurrenceKind::Daily,
        "weekly" => todomrs_core::domain::RecurrenceKind::Weekly,
        "monthly" => todomrs_core::domain::RecurrenceKind::Monthly,
        "yearly" => todomrs_core::domain::RecurrenceKind::Yearly,
        _ => todomrs_core::domain::RecurrenceKind::Daily,
    }
}

/// Deserialize an anchor mode string to its enum variant.
fn deserialize_anchor_mode(s: &str) -> todomrs_core::domain::AnchorMode {
    match s.to_lowercase().as_str() {
        "completion" => todomrs_core::domain::AnchorMode::Completion,
        _ => todomrs_core::domain::AnchorMode::Schedule,
    }
}
