//! Terminal User Interface for REAM daemon monitoring
//!
//! Provides a professional real-time monitoring dashboard using ratatui
//! for visualizing actor states, system metrics, and performance data.

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use std::io;

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::{Backend, CrosstermBackend},
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{
        Block, Borders, Gauge, List, ListItem, ListState,
        Paragraph, Row, Sparkline, Table, TableState, Tabs, Wrap,
    },
    Frame, Terminal,
};
use tui_input::{backend::crossterm::EventHandler, Input};

use crate::error::{ReamResult, ReamError};
use super::{ActorInfo, ActorStatus, SystemInfo};
use super::ipc::IpcClient;
use super::monitor::SystemMetrics;

/// TUI application state
pub struct TuiApp {
    /// IPC client for daemon communication
    client: IpcClient,
    /// Current tab index
    current_tab: usize,
    /// Actor list state
    actor_list_state: ListState,
    /// Actor table state
    actor_table_state: TableState,
    /// System information
    system_info: Option<SystemInfo>,
    /// Actor information
    actors: Vec<ActorInfo>,
    /// System metrics
    system_metrics: Option<SystemMetrics>,
    /// Selected actor for details
    selected_actor: Option<String>,
    /// Refresh interval
    refresh_interval: Duration,
    /// Last refresh time
    last_refresh: Instant,
    /// Input field for commands
    input: Input,
    /// Input mode
    input_mode: InputMode,
    /// Status message
    status_message: String,
    /// Error message
    error_message: Option<String>,
    /// Should quit
    should_quit: bool,
}

/// Input mode for the TUI
#[derive(Debug, Clone, PartialEq)]
pub enum InputMode {
    Normal,
    Command,
    Filter,
}

/// Tab names
const TAB_NAMES: &[&str] = &["Overview", "Actors", "Performance", "Logs", "Commands"];

impl TuiApp {
    /// Create a new TUI application
    pub fn new(socket_path: PathBuf, refresh_interval: Duration) -> Self {
        let client = IpcClient::new(socket_path);
        let mut actor_list_state = ListState::default();
        actor_list_state.select(Some(0));
        
        TuiApp {
            client,
            current_tab: 0,
            actor_list_state,
            actor_table_state: TableState::default(),
            system_info: None,
            actors: Vec::new(),
            system_metrics: None,
            selected_actor: None,
            refresh_interval,
            last_refresh: Instant::now(),
            input: Input::default(),
            input_mode: InputMode::Normal,
            status_message: "Ready".to_string(),
            error_message: None,
            should_quit: false,
        }
    }
    
    /// Run the TUI application
    pub async fn run(&mut self) -> ReamResult<()> {
        // Setup terminal
        enable_raw_mode().map_err(|e| ReamError::Io(e))?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)
            .map_err(|e| ReamError::Io(e))?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend).map_err(|e| ReamError::Io(e))?;
        
        // Initial data fetch
        self.refresh_data().await;
        
        // Main event loop
        let result = self.run_event_loop(&mut terminal).await;
        
        // Restore terminal
        disable_raw_mode().map_err(|e| ReamError::Io(e))?;
        execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        ).map_err(|e| ReamError::Io(e))?;
        terminal.show_cursor().map_err(|e| ReamError::Io(e))?;
        
        result
    }
    
    /// Main event loop
    async fn run_event_loop<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> ReamResult<()> {
        loop {
            // Draw the UI
            terminal.draw(|f| self.draw(f)).map_err(|e| ReamError::Io(e))?;
            
            // Handle events with timeout
            if event::poll(Duration::from_millis(100)).map_err(|e| ReamError::Io(e))? {
                if let Event::Key(key) = event::read().map_err(|e| ReamError::Io(e))? {
                    if key.kind == KeyEventKind::Press {
                        self.handle_key_event(key.code).await;
                    }
                }
            }
            
            // Refresh data if needed
            if self.last_refresh.elapsed() >= self.refresh_interval {
                self.refresh_data().await;
            }
            
            // Check if should quit
            if self.should_quit {
                break;
            }
        }
        
        Ok(())
    }
    
    /// Handle key events
    async fn handle_key_event(&mut self, key: KeyCode) {
        match self.input_mode {
            InputMode::Normal => self.handle_normal_key(key).await,
            InputMode::Command => self.handle_command_key(key).await,
            InputMode::Filter => self.handle_filter_key(key).await,
        }
    }
    
    /// Handle keys in normal mode
    async fn handle_normal_key(&mut self, key: KeyCode) {
        match key {
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Char('r') => self.refresh_data().await,
            KeyCode::Char(':') => {
                self.input_mode = InputMode::Command;
                self.input.reset();
            }
            KeyCode::Char('/') => {
                self.input_mode = InputMode::Filter;
                self.input.reset();
            }
            KeyCode::Tab => {
                self.current_tab = (self.current_tab + 1) % TAB_NAMES.len();
            }
            KeyCode::BackTab => {
                self.current_tab = if self.current_tab == 0 {
                    TAB_NAMES.len() - 1
                } else {
                    self.current_tab - 1
                };
            }
            KeyCode::Up => self.move_selection_up(),
            KeyCode::Down => self.move_selection_down(),
            KeyCode::Enter => self.select_current_item().await,
            _ => {}
        }
    }
    
    /// Handle keys in command mode
    async fn handle_command_key(&mut self, key: KeyCode) {
        match key {
            KeyCode::Enter => {
                let command = self.input.value().to_string();
                self.execute_command(command).await;
                self.input_mode = InputMode::Normal;
                self.input.reset();
            }
            KeyCode::Esc => {
                self.input_mode = InputMode::Normal;
                self.input.reset();
            }
            _ => {
                self.input.handle_event(&Event::Key(event::KeyEvent::new(
                    key,
                    event::KeyModifiers::empty(),
                )));
            }
        }
    }
    
    /// Handle keys in filter mode
    async fn handle_filter_key(&mut self, key: KeyCode) {
        match key {
            KeyCode::Enter => {
                // Apply filter
                self.input_mode = InputMode::Normal;
            }
            KeyCode::Esc => {
                self.input_mode = InputMode::Normal;
                self.input.reset();
            }
            _ => {
                self.input.handle_event(&Event::Key(event::KeyEvent::new(
                    key,
                    event::KeyModifiers::empty(),
                )));
            }
        }
    }
    
    /// Move selection up
    fn move_selection_up(&mut self) {
        match self.current_tab {
            1 => { // Actors tab
                let i = match self.actor_list_state.selected() {
                    Some(i) => {
                        if i == 0 {
                            self.actors.len() - 1
                        } else {
                            i - 1
                        }
                    }
                    None => 0,
                };
                self.actor_list_state.select(Some(i));
            }
            _ => {}
        }
    }
    
    /// Move selection down
    fn move_selection_down(&mut self) {
        match self.current_tab {
            1 => { // Actors tab
                let i = match self.actor_list_state.selected() {
                    Some(i) => {
                        if i >= self.actors.len() - 1 {
                            0
                        } else {
                            i + 1
                        }
                    }
                    None => 0,
                };
                self.actor_list_state.select(Some(i));
            }
            _ => {}
        }
    }
    
    /// Select current item
    async fn select_current_item(&mut self) {
        match self.current_tab {
            1 => { // Actors tab
                if let Some(i) = self.actor_list_state.selected() {
                    if let Some(actor) = self.actors.get(i) {
                        self.selected_actor = Some(actor.pid.to_string());
                        self.status_message = format!("Selected actor: {}", actor.pid);
                    }
                }
            }
            _ => {}
        }
    }
    
    /// Execute a command
    async fn execute_command(&mut self, command: String) {
        let parts: Vec<&str> = command.split_whitespace().collect();
        if parts.is_empty() {
            return;
        }
        
        match parts[0] {
            "kill" => {
                if parts.len() >= 2 {
                    let pid = parts[1].to_string();
                    let reason = parts.get(2).unwrap_or(&"normal").to_string();
                    match self.client.kill_actor(pid.clone(), reason).await {
                        Ok(msg) => self.status_message = msg,
                        Err(e) => self.error_message = Some(e.to_string()),
                    }
                }
            }
            "suspend" => {
                if parts.len() >= 2 {
                    let pid = parts[1].to_string();
                    match self.client.suspend_actor(pid.clone()).await {
                        Ok(msg) => self.status_message = msg,
                        Err(e) => self.error_message = Some(e.to_string()),
                    }
                }
            }
            "resume" => {
                if parts.len() >= 2 {
                    let pid = parts[1].to_string();
                    match self.client.resume_actor(pid.clone()).await {
                        Ok(msg) => self.status_message = msg,
                        Err(e) => self.error_message = Some(e.to_string()),
                    }
                }
            }
            "restart" => {
                if parts.len() >= 2 {
                    let pid = parts[1].to_string();
                    match self.client.restart_actor(pid.clone()).await {
                        Ok(msg) => self.status_message = msg,
                        Err(e) => self.error_message = Some(e.to_string()),
                    }
                }
            }
            "send" => {
                if parts.len() >= 3 {
                    let pid = parts[1].to_string();
                    let message = parts[2..].join(" ");
                    match self.client.send_actor_message(pid.clone(), message).await {
                        Ok(msg) => self.status_message = msg,
                        Err(e) => self.error_message = Some(e.to_string()),
                    }
                }
            }
            "refresh" => {
                self.refresh_data().await;
                self.status_message = "Data refreshed".to_string();
            }
            _ => {
                self.error_message = Some(format!("Unknown command: {}", parts[0]));
            }
        }
    }
    
    /// Refresh data from daemon
    async fn refresh_data(&mut self) {
        // Get system info
        match self.client.get_system_info().await {
            Ok(info) => {
                self.system_info = Some(info);
                self.error_message = None;
            }
            Err(e) => {
                self.error_message = Some(format!("Failed to get system info: {}", e));
            }
        }
        
        // Get actor list
        match self.client.list_actors(true).await {
            Ok(actors) => {
                self.actors = actors;
            }
            Err(e) => {
                self.error_message = Some(format!("Failed to get actors: {}", e));
            }
        }
        
        self.last_refresh = Instant::now();
    }
    
    /// Draw the UI
    fn draw(&mut self, f: &mut Frame) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Header
                Constraint::Min(0),    // Main content
                Constraint::Length(3), // Footer
            ])
            .split(f.size());
        
        // Draw header
        self.draw_header(f, chunks[0]);
        
        // Draw main content based on current tab
        match self.current_tab {
            0 => self.draw_overview(f, chunks[1]),
            1 => self.draw_actors(f, chunks[1]),
            2 => self.draw_performance(f, chunks[1]),
            3 => self.draw_logs(f, chunks[1]),
            4 => self.draw_commands(f, chunks[1]),
            _ => {}
        }
        
        // Draw footer
        self.draw_footer(f, chunks[2]);
    }
    
    /// Draw header with tabs
    fn draw_header(&self, f: &mut Frame, area: Rect) {
        let titles = TAB_NAMES
            .iter()
            .map(|t| Line::from(*t))
            .collect();
        
        let tabs = Tabs::new(titles)
            .block(Block::default().borders(Borders::ALL).title("REAM Daemon Monitor"))
            .select(self.current_tab)
            .style(Style::default().fg(Color::Cyan))
            .highlight_style(
                Style::default()
                    .add_modifier(Modifier::BOLD)
                    .bg(Color::Black)
                    .fg(Color::Yellow),
            );
        
        f.render_widget(tabs, area);
    }
    
    /// Draw footer with status and help
    fn draw_footer(&self, f: &mut Frame, area: Rect) {
        let footer_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
            .split(area);
        
        // Status message
        let status_text = match &self.error_message {
            Some(err) => Text::from(Line::from(vec![
                Span::styled("Error: ", Style::default().fg(Color::Red)),
                Span::raw(err),
            ])),
            None => Text::from(Line::from(vec![
                Span::styled("Status: ", Style::default().fg(Color::Green)),
                Span::raw(&self.status_message),
            ])),
        };
        
        let status = Paragraph::new(status_text)
            .block(Block::default().borders(Borders::ALL).title("Status"));
        f.render_widget(status, footer_chunks[0]);
        
        // Help text
        let help_text = match self.input_mode {
            InputMode::Normal => "q:quit r:refresh Tab:switch /:filter ::command",
            InputMode::Command => "Enter:execute Esc:cancel",
            InputMode::Filter => "Enter:apply Esc:cancel",
        };
        
        let help = Paragraph::new(help_text)
            .block(Block::default().borders(Borders::ALL).title("Help"))
            .alignment(Alignment::Center);
        f.render_widget(help, footer_chunks[1]);
    }

    /// Draw overview tab
    fn draw_overview(&self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(7),  // System info
                Constraint::Length(7),  // Resource usage
                Constraint::Min(0),     // Actor summary
            ])
            .split(area);

        // System information
        if let Some(ref system_info) = self.system_info {
            let system_text = vec![
                Line::from(format!("Uptime: {:?}", system_info.uptime)),
                Line::from(format!("Total Actors: {}", system_info.total_actors)),
                Line::from(format!("Active: {} | Suspended: {} | Crashed: {}",
                    system_info.active_actors,
                    system_info.suspended_actors,
                    system_info.crashed_actors)),
                Line::from(format!("Memory: {} bytes", system_info.total_memory)),
                Line::from(format!("Message Rate: {:.2} msg/s", system_info.system_message_rate)),
            ];

            let system_info_widget = Paragraph::new(system_text)
                .block(Block::default().borders(Borders::ALL).title("System Information"))
                .wrap(Wrap { trim: true });
            f.render_widget(system_info_widget, chunks[0]);

            // Resource usage gauges
            let resource_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(33), Constraint::Percentage(33), Constraint::Percentage(34)])
                .split(chunks[1]);

            let cpu_gauge = Gauge::default()
                .block(Block::default().borders(Borders::ALL).title("CPU"))
                .gauge_style(Style::default().fg(Color::Yellow))
                .percent((system_info.cpu_usage * 100.0) as u16);
            f.render_widget(cpu_gauge, resource_chunks[0]);

            let memory_gauge = Gauge::default()
                .block(Block::default().borders(Borders::ALL).title("Memory"))
                .gauge_style(Style::default().fg(Color::Blue))
                .percent((system_info.memory_usage_percent * 100.0) as u16);
            f.render_widget(memory_gauge, resource_chunks[1]);

            let load_gauge = Gauge::default()
                .block(Block::default().borders(Borders::ALL).title("Load"))
                .gauge_style(Style::default().fg(Color::Green))
                .percent((system_info.load_average * 10.0) as u16);
            f.render_widget(load_gauge, resource_chunks[2]);
        }

        // Actor status summary
        let actor_status_counts = self.get_actor_status_counts();
        let status_items: Vec<ListItem> = actor_status_counts
            .iter()
            .map(|(status, count)| {
                let color = match status.as_str() {
                    "Running" => Color::Green,
                    "Suspended" => Color::Yellow,
                    "Crashed" => Color::Red,
                    "Terminated" => Color::Gray,
                    _ => Color::White,
                };
                ListItem::new(format!("{}: {}", status, count))
                    .style(Style::default().fg(color))
            })
            .collect();

        let status_list = List::new(status_items)
            .block(Block::default().borders(Borders::ALL).title("Actor Status Summary"));
        f.render_widget(status_list, chunks[2]);
    }

    /// Draw actors tab
    fn draw_actors(&self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
            .split(area);

        // Actor list
        let actor_items: Vec<ListItem> = self.actors
            .iter()
            .map(|actor| {
                let status_color = match actor.status {
                    ActorStatus::Running => Color::Green,
                    ActorStatus::Suspended => Color::Yellow,
                    ActorStatus::Crashed => Color::Red,
                    ActorStatus::Terminated => Color::Gray,
                    _ => Color::White,
                };

                ListItem::new(format!(
                    "{} | {} | {} msgs | {} bytes",
                    actor.pid,
                    format!("{:?}", actor.status),
                    actor.mailbox_size,
                    actor.memory_usage
                )).style(Style::default().fg(status_color))
            })
            .collect();

        let actor_list = List::new(actor_items)
            .block(Block::default().borders(Borders::ALL).title("Actors"))
            .highlight_style(Style::default().add_modifier(Modifier::BOLD).bg(Color::DarkGray));
        f.render_stateful_widget(actor_list, chunks[0], &mut self.actor_list_state.clone());

        // Actor details
        if let Some(selected_idx) = self.actor_list_state.selected() {
            if let Some(actor) = self.actors.get(selected_idx) {
                let details_text = vec![
                    Line::from(format!("PID: {}", actor.pid)),
                    Line::from(format!("Status: {:?}", actor.status)),
                    Line::from(format!("Type: {}", actor.actor_type)),
                    Line::from(format!("Uptime: {:?}", actor.uptime)),
                    Line::from(format!("Mailbox: {}", actor.mailbox_size)),
                    Line::from(format!("Memory: {} bytes", actor.memory_usage)),
                    Line::from(format!("Messages: {}", actor.messages_processed)),
                    Line::from(format!("Rate: {:.2} msg/s", actor.message_rate)),
                    Line::from(format!("CPU Time: {} μs", actor.cpu_time)),
                    Line::from(format!("State: {}", actor.state_description)),
                    Line::from(format!("Links: {}", actor.links.len())),
                    Line::from(format!("Monitors: {}", actor.monitors.len())),
                ];

                let details = Paragraph::new(details_text)
                    .block(Block::default().borders(Borders::ALL).title("Actor Details"))
                    .wrap(Wrap { trim: true });
                f.render_widget(details, chunks[1]);
            }
        }
    }

    /// Draw performance tab
    fn draw_performance(&self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area);

        // Message rate chart (placeholder)
        let message_rates: Vec<u64> = self.actors
            .iter()
            .map(|actor| actor.message_rate as u64)
            .collect();

        if !message_rates.is_empty() {
            let sparkline = Sparkline::default()
                .block(Block::default().borders(Borders::ALL).title("Message Rates"))
                .data(&message_rates)
                .style(Style::default().fg(Color::Yellow));
            f.render_widget(sparkline, chunks[0]);
        }

        // Performance metrics table
        let total_message_rate = format!("{:.2} msg/s",
            self.actors.iter().map(|a| a.message_rate).sum::<f64>());
        let avg_memory = format!("{:.2} KB",
            self.actors.iter().map(|a| a.memory_usage).sum::<usize>() as f64 / 1024.0);
        let active_actors = self.actors.iter()
            .filter(|a| a.status == ActorStatus::Running).count().to_string();

        let performance_data = vec![
            Row::new(vec!["Total Message Rate", &total_message_rate]),
            Row::new(vec!["Average Memory", &avg_memory]),
            Row::new(vec!["Active Actors", &active_actors]),
        ];

        let performance_table = Table::new(performance_data)
            .block(Block::default().borders(Borders::ALL).title("Performance Metrics"))
            .header(Row::new(vec!["Metric", "Value"]).style(Style::default().add_modifier(Modifier::BOLD)))
            .widths(&[Constraint::Percentage(50), Constraint::Percentage(50)]);
        f.render_widget(performance_table, chunks[1]);
    }

    /// Draw logs tab
    fn draw_logs(&self, f: &mut Frame, area: Rect) {
        let log_text = vec![
            Line::from("Log viewing not yet implemented"),
            Line::from("This will show real-time logs from actors"),
            Line::from("and system events"),
        ];

        let logs = Paragraph::new(log_text)
            .block(Block::default().borders(Borders::ALL).title("Logs"))
            .wrap(Wrap { trim: true });
        f.render_widget(logs, area);
    }

    /// Draw commands tab
    fn draw_commands(&self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(3)])
            .split(area);

        // Command help
        let help_text = vec![
            Line::from("Available Commands:"),
            Line::from(""),
            Line::from("kill <pid> [reason]  - Kill an actor"),
            Line::from("suspend <pid>        - Suspend an actor"),
            Line::from("resume <pid>         - Resume an actor"),
            Line::from("restart <pid>        - Restart an actor"),
            Line::from("send <pid> <msg>     - Send message to actor"),
            Line::from("refresh              - Refresh data"),
            Line::from(""),
            Line::from("Press ':' to enter command mode"),
        ];

        let help = Paragraph::new(help_text)
            .block(Block::default().borders(Borders::ALL).title("Commands"))
            .wrap(Wrap { trim: true });
        f.render_widget(help, chunks[0]);

        // Command input
        if self.input_mode == InputMode::Command {
            let input_text = format!(": {}", self.input.value());
            let input_widget = Paragraph::new(input_text)
                .block(Block::default().borders(Borders::ALL).title("Command Input"));
            f.render_widget(input_widget, chunks[1]);
        }
    }

    /// Get actor status counts
    fn get_actor_status_counts(&self) -> HashMap<String, usize> {
        let mut counts = HashMap::new();
        for actor in &self.actors {
            let status_str = format!("{:?}", actor.status);
            *counts.entry(status_str).or_insert(0) += 1;
        }
        counts
    }
}
