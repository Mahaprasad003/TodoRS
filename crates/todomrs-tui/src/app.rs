use anyhow::Result;
use chrono::{Datelike, NaiveTime, Timelike, Weekday};
use crossterm::event::{Event, KeyCode, KeyEventKind, KeyModifiers};
use todomrs_core::domain::{Priority, Project, Task, TaskStatus};
use todomrs_core::NaturalLanguageParser;
use todomrs_store::{OperationStore, ProjectStore, TaskStore};
use todomrs_sync::operations::Operation;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq)]
pub enum View {
    Inbox,
    Today,
    Upcoming,
    Projects,
    Completed,
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
    pub status_message: Option<String>,
    pub project_counts: Vec<(Uuid, String, usize, usize)>,
    pub selected_project_id: Option<Uuid>,
    pub project_selected_index: usize,
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
            status_message: None,
            project_counts: Vec::new(),
            selected_project_id: None,
            project_selected_index: 0,
        }
    }

    /// Load tasks from the database for the current user.
    pub async fn refresh_tasks(&mut self) -> Result<()> {
        self.tasks = self.task_store.get_all(self.user_id).await?;
        self.refresh_project_counts().await?;
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
                                self.status_message = Some(format!("Created project: {}", name));
                                self.refresh_project_counts().await?;
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
                recurrence_rule_id: None,
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
            parts.join(", ")
        };

        self.status_message = Some(format!("Updated {}", changed_parts));
        self.refresh_tasks().await?;
        Ok(())
    }

    /// Parse the input buffer, create a task and operation, persist both.
    async fn create_task_from_input(&mut self) -> Result<()> {
        let input = self.input_buffer.trim().to_string();
        if input.is_empty() {
            return Ok(());
        }

        let (mut task, _recurrence_rule) =
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

        // Persist task
        self.task_store.create(&task).await?;

        // Record operation for sync
        let seq = self.op_store.get_next_seq(self.user_id, self.device_id).await?;
        let op = Operation::create_task(self.user_id, self.device_id, seq, &task);
        self.op_store.append(&op).await?;

        self.status_message = Some(format!("Created: {}", task.title));
        self.refresh_tasks().await?;
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

        self.status_message = Some(format!("{}: {}", description, task.title));
        self.refresh_tasks().await?;
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
        Ok(())
    }
}

/// Format a DateTime for natural-language editing (parser-friendly).
fn format_datetime_for_edit(dt: chrono::DateTime<chrono::Utc>) -> String {
    use chrono::Duration;

    let today = chrono::Utc::now().date_naive();
    let date = dt.date_naive();
    let time = dt.time();

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
