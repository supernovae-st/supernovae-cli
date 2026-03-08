//! TUI infrastructure for interactive modes.
//!
//! This module provides the explore command TUI.

use std::io::{self, Stdout};

use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Tabs},
    Frame, Terminal as RatatuiTerminal,
};

use crate::status::{credentials, mcp, ollama};

/// Format bytes as human-readable size string.
fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

/// Terminal wrapper that handles setup/teardown.
pub struct Terminal {
    terminal: RatatuiTerminal<CrosstermBackend<Stdout>>,
}

impl Terminal {
    /// Create a new terminal with alternate screen.
    pub fn new() -> io::Result<Self> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let terminal = RatatuiTerminal::new(backend)?;
        Ok(Self { terminal })
    }

    /// Get mutable reference to inner terminal.
    pub fn inner(&mut self) -> &mut RatatuiTerminal<CrosstermBackend<Stdout>> {
        &mut self.terminal
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(self.terminal.backend_mut(), LeaveAlternateScreen);
    }
}

/// Read a key event with timeout (100ms).
pub fn read_key() -> io::Result<Option<(KeyCode, KeyModifiers)>> {
    if event::poll(std::time::Duration::from_millis(100))? {
        if let Event::Key(key) = event::read()? {
            return Ok(Some((key.code, key.modifiers)));
        }
    }
    Ok(None)
}

/// Calculate a centered rect given percentage dimensions.
pub fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_width = r.width * percent_x / 100;
    let popup_height = r.height * percent_y / 100;
    let popup_x = (r.width.saturating_sub(popup_width)) / 2;
    let popup_y = (r.height.saturating_sub(popup_height)) / 2;
    Rect::new(r.x + popup_x, r.y + popup_y, popup_width, popup_height)
}

// =============================================================================
// EXPLORE APP
// =============================================================================

/// Resource categories.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Category {
    Models,
    Providers,
    McpServers,
    Skills,
}

impl Category {
    const ALL: [Category; 4] = [
        Category::Models,
        Category::Providers,
        Category::McpServers,
        Category::Skills,
    ];

    fn title(&self) -> &'static str {
        match self {
            Category::Models => "Models",
            Category::Providers => "Providers",
            Category::McpServers => "MCP Servers",
            Category::Skills => "Skills",
        }
    }

    fn icon(&self) -> &'static str {
        match self {
            Category::Models => "M",
            Category::Providers => "P",
            Category::McpServers => "S",
            Category::Skills => "K",
        }
    }
}

/// A resource item in a category.
#[derive(Debug, Clone)]
pub struct ResourceItem {
    pub name: String,
    pub status: String,
    pub details: String,
}

/// Main explore application state.
pub struct ExploreApp {
    category: usize,
    list_state: ListState,
    items: Vec<ResourceItem>,
    show_help: bool,
}

impl ExploreApp {
    /// Create a new explore app, loading initial data.
    pub async fn new() -> Self {
        let mut app = Self {
            category: 0,
            list_state: ListState::default(),
            items: Vec::new(),
            show_help: false,
        };
        app.load_items().await;
        if !app.items.is_empty() {
            app.list_state.select(Some(0));
        }
        app
    }

    /// Load items for the current category.
    async fn load_items(&mut self) {
        self.items = match Category::ALL[self.category] {
            Category::Models => self.load_models().await,
            Category::Providers => self.load_providers().await,
            Category::McpServers => self.load_mcp_servers().await,
            Category::Skills => self.load_skills().await,
        };
        self.list_state
            .select(if self.items.is_empty() { None } else { Some(0) });
    }

    async fn load_models(&self) -> Vec<ResourceItem> {
        let status = ollama::collect().await;
        if status.models.is_empty() {
            vec![ResourceItem {
                name: "No models found".into(),
                status: if status.running { "empty" } else { "offline" }.into(),
                details: if status.running {
                    "Run 'ollama pull <model>' to download".into()
                } else {
                    "Run 'ollama serve' to start".into()
                },
            }]
        } else {
            status
                .models
                .into_iter()
                .map(|m| ResourceItem {
                    name: m.name,
                    status: if m.loaded { "loaded" } else { "ready" }.into(),
                    details: format_size(m.size),
                })
                .collect()
        }
    }

    async fn load_providers(&self) -> Vec<ResourceItem> {
        let providers = credentials::collect().await;
        if providers.is_empty() {
            vec![ResourceItem {
                name: "No providers configured".into(),
                status: "none".into(),
                details: "Run 'spn provider set <name>' to add".into(),
            }]
        } else {
            providers
                .into_iter()
                .map(|p| {
                    let status = match p.status {
                        credentials::Status::Ready => "configured",
                        credentials::Status::Local => "local",
                        credentials::Status::NotSet => "missing",
                    };
                    let source = p
                        .source
                        .map(|s| match s {
                            credentials::Source::Keychain => "keychain",
                            credentials::Source::Env => "env",
                            credentials::Source::DotEnv => ".env",
                            credentials::Source::Local => "local",
                        })
                        .unwrap_or("-");
                    ResourceItem {
                        name: p.name,
                        status: status.into(),
                        details: source.into(),
                    }
                })
                .collect()
        }
    }

    async fn load_mcp_servers(&self) -> Vec<ResourceItem> {
        let servers = mcp::collect().await;
        if servers.is_empty() {
            vec![ResourceItem {
                name: "No MCP servers configured".into(),
                status: "none".into(),
                details: "Run 'spn mcp add <name>' to add one".into(),
            }]
        } else {
            servers
                .into_iter()
                .map(|s| {
                    let status = match s.status {
                        mcp::ServerStatus::Connected => "connected",
                        mcp::ServerStatus::Starting => "starting",
                        mcp::ServerStatus::Ready => "ready",
                        mcp::ServerStatus::Disabled => "disabled",
                        mcp::ServerStatus::Error => "error",
                    };
                    ResourceItem {
                        name: s.name,
                        status: status.into(),
                        details: s.command,
                    }
                })
                .collect()
        }
    }

    async fn load_skills(&self) -> Vec<ResourceItem> {
        vec![ResourceItem {
            name: "Skills browser".into(),
            status: "coming soon".into(),
            details: "Will list installed skills from spn.yaml".into(),
        }]
    }

    /// Run the TUI event loop.
    pub async fn run(&mut self) -> io::Result<()> {
        let mut terminal = Terminal::new()?;

        loop {
            terminal.inner().draw(|f| self.render(f))?;

            if let Some((key, modifiers)) = read_key()? {
                match (key, modifiers) {
                    (KeyCode::Char('q'), _) | (KeyCode::Esc, _) => {
                        if self.show_help {
                            self.show_help = false;
                        } else {
                            break;
                        }
                    }
                    (KeyCode::Char('?'), _) => {
                        self.show_help = !self.show_help;
                    }
                    (KeyCode::Tab, KeyModifiers::NONE) => {
                        self.category = (self.category + 1) % Category::ALL.len();
                        self.load_items().await;
                    }
                    (KeyCode::BackTab, _) => {
                        self.category = if self.category == 0 {
                            Category::ALL.len() - 1
                        } else {
                            self.category - 1
                        };
                        self.load_items().await;
                    }
                    (KeyCode::Down, _) | (KeyCode::Char('j'), _) => {
                        self.next_item();
                    }
                    (KeyCode::Up, _) | (KeyCode::Char('k'), _) => {
                        self.prev_item();
                    }
                    _ => {}
                }
            }
        }

        Ok(())
    }

    fn next_item(&mut self) {
        if self.items.is_empty() {
            return;
        }
        let i = match self.list_state.selected() {
            Some(i) => (i + 1) % self.items.len(),
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    fn prev_item(&mut self) {
        if self.items.is_empty() {
            return;
        }
        let i = match self.list_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.items.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.list_state.select(Some(i));
    }

    fn render(&mut self, f: &mut Frame<'_>) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(5),
                Constraint::Length(1),
            ])
            .split(f.area());

        self.render_tabs(f, chunks[0]);
        self.render_content(f, chunks[1]);
        self.render_status(f, chunks[2]);

        if self.show_help {
            self.render_help(f);
        }
    }

    fn render_tabs(&self, f: &mut Frame<'_>, area: Rect) {
        let titles: Vec<Line> = Category::ALL
            .iter()
            .map(|c| Line::from(format!("[{}] {}", c.icon(), c.title())))
            .collect();

        let tabs = Tabs::new(titles)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" spn explore "),
            )
            .select(self.category)
            .style(Style::default().fg(Color::White))
            .highlight_style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            );

        f.render_widget(tabs, area);
    }

    fn render_content(&mut self, f: &mut Frame<'_>, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area);

        // List view
        let items: Vec<ListItem> = self
            .items
            .iter()
            .map(|item| {
                let status_color = match item.status.as_str() {
                    "configured" | "loaded" | "connected" | "local" => Color::Green,
                    "missing" | "disabled" | "offline" | "error" => Color::Red,
                    "ready" | "starting" => Color::Yellow,
                    _ => Color::DarkGray,
                };
                ListItem::new(Line::from(vec![
                    Span::styled(&item.name, Style::default().fg(Color::White)),
                    Span::raw(" "),
                    Span::styled(
                        format!("[{}]", &item.status),
                        Style::default().fg(status_color),
                    ),
                ]))
            })
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(format!(" {} ", Category::ALL[self.category].title())),
            )
            .highlight_style(
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("> ");

        f.render_stateful_widget(list, chunks[0], &mut self.list_state);

        // Details view
        let details = if let Some(selected) = self.list_state.selected() {
            if let Some(item) = self.items.get(selected) {
                vec![
                    Line::from(vec![
                        Span::styled("Name: ", Style::default().fg(Color::DarkGray)),
                        Span::raw(&item.name),
                    ]),
                    Line::from(vec![
                        Span::styled("Status: ", Style::default().fg(Color::DarkGray)),
                        Span::raw(&item.status),
                    ]),
                    Line::from(vec![
                        Span::styled("Details: ", Style::default().fg(Color::DarkGray)),
                        Span::raw(&item.details),
                    ]),
                ]
            } else {
                vec![Line::from("Select an item")]
            }
        } else {
            vec![Line::from("No items available")]
        };

        let details_widget = Paragraph::new(details)
            .block(Block::default().borders(Borders::ALL).title(" Details "));

        f.render_widget(details_widget, chunks[1]);
    }

    fn render_status(&self, f: &mut Frame<'_>, area: Rect) {
        let help_text = "Tab: switch category | Up/Down: navigate | ?: help | q: quit";
        let status = Paragraph::new(help_text).style(Style::default().fg(Color::DarkGray));
        f.render_widget(status, area);
    }

    fn render_help(&self, f: &mut Frame<'_>) {
        let area = centered_rect(60, 60, f.area());
        f.render_widget(Clear, area);

        let help_text = vec![
            Line::from(""),
            Line::from(Span::styled(
                "  Keyboard Shortcuts",
                Style::default().add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from("  Tab / Shift+Tab    Switch category"),
            Line::from("  Up / Down / j / k  Navigate list"),
            Line::from("  ?                  Toggle this help"),
            Line::from("  q / Esc            Quit"),
            Line::from(""),
        ];

        let help =
            Paragraph::new(help_text).block(Block::default().borders(Borders::ALL).title(" Help "));

        f.render_widget(help, area);
    }
}
