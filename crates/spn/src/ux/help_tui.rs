//! Interactive TUI help screen for spn CLI.
//!
//! Provides a navigable help screen with keyboard navigation,
//! showing commands organized by category with descriptions.
//!
//! # Usage
//!
//! ```bash
//! spn --interactive    # or spn -i
//! spn help --tui
//! ```
//!
//! # Keyboard Controls
//!
//! - `j`/`k` or Arrow keys: Navigate sections/commands
//! - `Enter`: Expand/collapse section
//! - `q`/`Esc`: Exit
//! - `?`: Show help overlay

use std::io::{self, Stdout};

use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, Clear, List, ListItem, ListState, Paragraph, Scrollbar,
        ScrollbarOrientation, ScrollbarState,
    },
    Frame, Terminal,
};

// =============================================================================
// CONSTANTS
// =============================================================================

const VERSION: &str = env!("CARGO_PKG_VERSION");

// =============================================================================
// DATA STRUCTURES
// =============================================================================

/// A command with its name and description.
#[derive(Debug, Clone)]
struct Command {
    name: &'static str,
    desc: &'static str,
}

impl Command {
    const fn new(name: &'static str, desc: &'static str) -> Self {
        Self { name, desc }
    }
}

/// A section containing related commands.
#[derive(Debug, Clone)]
struct Section {
    name: &'static str,
    icon: &'static str,
    color: Color,
    commands: &'static [Command],
}

impl Section {
    const fn new(
        name: &'static str,
        icon: &'static str,
        color: Color,
        commands: &'static [Command],
    ) -> Self {
        Self {
            name,
            icon,
            color,
            commands,
        }
    }
}

// =============================================================================
// SECTION DATA
// =============================================================================

static SECTIONS: &[Section] = &[
    Section::new(
        "Get Started",
        "R",
        Color::Green,
        &[
            Command::new("setup", "Interactive onboarding wizard"),
            Command::new("setup nika", "Install Nika workflow engine"),
            Command::new("setup novanet", "Install NovaNet knowledge graph"),
            Command::new("doctor", "Verify installation health"),
            Command::new("tour", "Guided feature walkthrough"),
        ],
    ),
    Section::new(
        "Providers",
        "P",
        Color::Yellow,
        &[
            Command::new("provider list", "Show all providers and key status"),
            Command::new("provider set <name>", "Store API key in OS Keychain"),
            Command::new("provider get <name>", "Retrieve key (masked)"),
            Command::new("provider delete <name>", "Remove key from keychain"),
            Command::new("provider migrate", "Move env vars to keychain"),
            Command::new("provider test <name>", "Validate key format"),
            Command::new("provider status", "Full diagnostic"),
        ],
    ),
    Section::new(
        "Models",
        "M",
        Color::Magenta,
        &[
            Command::new("model list", "List installed Ollama models"),
            Command::new("model pull <name>", "Download model from registry"),
            Command::new("model load <name>", "Load model into VRAM"),
            Command::new("model unload <name>", "Release model from memory"),
            Command::new("model delete <name>", "Delete local model"),
            Command::new("model status", "Show running models + VRAM"),
            Command::new("model search <q>", "Search available models"),
            Command::new("model info <name>", "Show model details"),
            Command::new("model recommend", "Get model recommendations"),
            Command::new("model run <model>", "Quick inference"),
        ],
    ),
    Section::new(
        "MCP Servers",
        "S",
        Color::Blue,
        &[
            Command::new("mcp add <name>", "Add MCP server (44 aliases)"),
            Command::new("mcp remove <name>", "Remove MCP server"),
            Command::new("mcp list", "Show all configured servers"),
            Command::new("mcp test <name>", "Test server connection"),
            Command::new("mcp logs <name>", "View server logs"),
            Command::new("mcp serve", "Start REST-to-MCP server"),
            Command::new("mcp wrap", "Wrap REST API as MCP (wizard)"),
        ],
    ),
    Section::new(
        "Packages",
        "K",
        Color::Cyan,
        &[
            Command::new("add <package>", "Add package to project"),
            Command::new("remove <package>", "Remove package"),
            Command::new("install", "Install from spn.yaml"),
            Command::new("update", "Update to latest versions"),
            Command::new("list", "Show installed packages"),
            Command::new("search <query>", "Search registry"),
            Command::new("info <package>", "Show package details"),
            Command::new("outdated", "Check for updates"),
        ],
    ),
    Section::new(
        "Skills",
        "L",
        Color::Green,
        &[
            Command::new("skill add <name>", "Install from skills.sh"),
            Command::new("skill remove <name>", "Uninstall skill"),
            Command::new("skill list", "Show installed skills"),
            Command::new("skill search <q>", "Browse 57K+ skills"),
        ],
    ),
    Section::new(
        "Jobs",
        "J",
        Color::Cyan,
        &[
            Command::new("jobs list", "List background jobs"),
            Command::new("jobs submit <wf>", "Queue workflow for execution"),
            Command::new("jobs status <id>", "Check job status"),
            Command::new("jobs cancel <id>", "Abort running job"),
            Command::new("jobs output <id>", "View job stdout"),
            Command::new("jobs clear", "Cleanup old jobs"),
        ],
    ),
    Section::new(
        "System",
        "Y",
        Color::White,
        &[
            Command::new("status", "System dashboard"),
            Command::new("sync", "Sync packages to editors"),
            Command::new("config show", "Show configuration"),
            Command::new("config where", "Show config file paths"),
            Command::new("config edit", "Edit configuration"),
            Command::new("init", "Initialize new project"),
            Command::new("explore", "Interactive TUI browser"),
            Command::new("suggest", "Smart help suggestions"),
            Command::new("daemon start|stop", "Manage background daemon"),
            Command::new("completion install", "Install shell completions"),
        ],
    ),
    Section::new(
        "Ecosystem",
        "E",
        Color::Cyan,
        &[
            Command::new("nk <args>", "Proxy to Nika CLI"),
            Command::new("nv <args>", "Proxy to NovaNet CLI"),
            Command::new("backup create", "Create backup archive"),
            Command::new("backup restore", "Restore from backup"),
        ],
    ),
];

// =============================================================================
// TUI STATE
// =============================================================================

/// Navigation item - either a section header or a command.
#[derive(Debug, Clone)]
enum NavItem {
    Section {
        index: usize,
        expanded: bool,
    },
    Command {
        section_index: usize,
        cmd_index: usize,
    },
}

/// Main TUI application state.
pub struct HelpTui {
    /// All navigation items (sections + commands when expanded)
    items: Vec<NavItem>,
    /// Current selection index
    selected: usize,
    /// List state for ratatui
    list_state: ListState,
    /// Scroll state for scrollbar
    scroll_state: ScrollbarState,
    /// Which sections are expanded
    expanded: Vec<bool>,
    /// Show help overlay
    show_help: bool,
}

impl HelpTui {
    /// Create a new help TUI with all sections collapsed.
    pub fn new() -> Self {
        let expanded = vec![false; SECTIONS.len()];
        let mut tui = Self {
            items: Vec::new(),
            selected: 0,
            list_state: ListState::default(),
            scroll_state: ScrollbarState::default(),
            expanded,
            show_help: false,
        };
        tui.rebuild_items();
        tui.list_state.select(Some(0));
        tui
    }

    /// Rebuild the navigation items based on expansion state.
    fn rebuild_items(&mut self) {
        self.items.clear();
        for (idx, section) in SECTIONS.iter().enumerate() {
            let expanded = self.expanded[idx];
            self.items.push(NavItem::Section {
                index: idx,
                expanded,
            });
            if expanded {
                for cmd_idx in 0..section.commands.len() {
                    self.items.push(NavItem::Command {
                        section_index: idx,
                        cmd_index: cmd_idx,
                    });
                }
            }
        }
        self.scroll_state = self.scroll_state.content_length(self.items.len());
    }

    /// Toggle expansion of the currently selected section.
    fn toggle_expand(&mut self) {
        if let Some(NavItem::Section { index, .. }) = self.items.get(self.selected) {
            self.expanded[*index] = !self.expanded[*index];
            self.rebuild_items();
        }
    }

    /// Move selection down.
    fn next(&mut self) {
        if self.items.is_empty() {
            return;
        }
        self.selected = (self.selected + 1) % self.items.len();
        self.list_state.select(Some(self.selected));
        self.scroll_state = self.scroll_state.position(self.selected);
    }

    /// Move selection up.
    fn prev(&mut self) {
        if self.items.is_empty() {
            return;
        }
        self.selected = if self.selected == 0 {
            self.items.len() - 1
        } else {
            self.selected - 1
        };
        self.list_state.select(Some(self.selected));
        self.scroll_state = self.scroll_state.position(self.selected);
    }

    /// Run the TUI event loop.
    pub fn run(&mut self) -> io::Result<()> {
        // Setup terminal
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        let result = self.event_loop(&mut terminal);

        // Cleanup terminal
        disable_raw_mode()?;
        execute!(terminal.backend_mut(), LeaveAlternateScreen)?;

        result
    }

    /// Main event loop.
    fn event_loop(&mut self, terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> io::Result<()> {
        loop {
            terminal.draw(|f| self.render(f))?;

            // Poll for events with 100ms timeout
            if event::poll(std::time::Duration::from_millis(100))? {
                if let Event::Key(key) = event::read()? {
                    // Only handle key press events, not release
                    if key.kind != KeyEventKind::Press {
                        continue;
                    }

                    match (key.code, key.modifiers) {
                        // Quit
                        (KeyCode::Char('q'), _) | (KeyCode::Esc, _) => {
                            if self.show_help {
                                self.show_help = false;
                            } else {
                                break;
                            }
                        }
                        // Help overlay
                        (KeyCode::Char('?'), _) => {
                            self.show_help = !self.show_help;
                        }
                        // Navigation
                        (KeyCode::Down, _) | (KeyCode::Char('j'), _) => {
                            self.next();
                        }
                        (KeyCode::Up, _) | (KeyCode::Char('k'), _) => {
                            self.prev();
                        }
                        // Expand/collapse
                        (KeyCode::Enter, _) | (KeyCode::Char(' '), _) => {
                            self.toggle_expand();
                        }
                        // Expand all
                        (KeyCode::Char('e'), KeyModifiers::NONE) => {
                            for exp in &mut self.expanded {
                                *exp = true;
                            }
                            self.rebuild_items();
                        }
                        // Collapse all
                        (KeyCode::Char('c'), KeyModifiers::NONE) => {
                            for exp in &mut self.expanded {
                                *exp = false;
                            }
                            self.rebuild_items();
                        }
                        _ => {}
                    }
                }
            }
        }
        Ok(())
    }

    /// Render the TUI.
    fn render(&mut self, f: &mut Frame<'_>) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Header
                Constraint::Min(5),    // Content
                Constraint::Length(1), // Footer
            ])
            .split(f.area());

        self.render_header(f, chunks[0]);
        self.render_content(f, chunks[1]);
        self.render_footer(f, chunks[2]);

        if self.show_help {
            self.render_help_overlay(f);
        }
    }

    /// Render the header with title and version.
    fn render_header(&self, f: &mut Frame<'_>, area: Rect) {
        let title = format!(" spn v{} - The Agentic AI Toolkit ", VERSION);
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan))
            .title(Span::styled(
                title,
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ));
        f.render_widget(block, area);
    }

    /// Render the main content area with sections list and details.
    fn render_content(&mut self, f: &mut Frame<'_>, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
            .split(area);

        self.render_sections_list(f, chunks[0]);
        self.render_details(f, chunks[1]);
    }

    /// Render the sections/commands list on the left.
    fn render_sections_list(&mut self, f: &mut Frame<'_>, area: Rect) {
        let items: Vec<ListItem> = self
            .items
            .iter()
            .map(|item| match item {
                NavItem::Section { index, expanded } => {
                    let section = &SECTIONS[*index];
                    let arrow = if *expanded { "v" } else { ">" };
                    let text = format!("{} [{}] {}", arrow, section.icon, section.name);
                    ListItem::new(Line::from(vec![Span::styled(
                        text,
                        Style::default()
                            .fg(section.color)
                            .add_modifier(Modifier::BOLD),
                    )]))
                }
                NavItem::Command {
                    section_index,
                    cmd_index,
                } => {
                    let section = &SECTIONS[*section_index];
                    let cmd = &section.commands[*cmd_index];
                    let text = format!("    {}", cmd.name);
                    ListItem::new(Line::from(vec![Span::styled(
                        text,
                        Style::default().fg(Color::White),
                    )]))
                }
            })
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::DarkGray))
                    .title(" Sections "),
            )
            .highlight_style(
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("> ");

        f.render_stateful_widget(list, area, &mut self.list_state);

        // Render scrollbar
        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("^"))
            .end_symbol(Some("v"));
        f.render_stateful_widget(
            scrollbar,
            area.inner(ratatui::layout::Margin {
                vertical: 1,
                horizontal: 0,
            }),
            &mut self.scroll_state,
        );
    }

    /// Render the details panel on the right.
    fn render_details(&self, f: &mut Frame<'_>, area: Rect) {
        let content = match self.items.get(self.selected) {
            Some(NavItem::Section { index, .. }) => {
                let section = &SECTIONS[*index];
                let mut lines = vec![
                    Line::from(""),
                    Line::from(vec![
                        Span::styled(
                            format!("[{}] ", section.icon),
                            Style::default().fg(section.color),
                        ),
                        Span::styled(
                            section.name,
                            Style::default()
                                .fg(section.color)
                                .add_modifier(Modifier::BOLD),
                        ),
                    ]),
                    Line::from(""),
                    Line::from(vec![Span::styled(
                        format!("{} commands", section.commands.len()),
                        Style::default().fg(Color::DarkGray),
                    )]),
                    Line::from(""),
                    Line::from(vec![Span::styled(
                        "Press Enter to expand",
                        Style::default().fg(Color::DarkGray),
                    )]),
                    Line::from(""),
                ];

                // Show command preview
                lines.push(Line::from(vec![Span::styled(
                    "Commands:",
                    Style::default()
                        .fg(Color::White)
                        .add_modifier(Modifier::BOLD),
                )]));
                lines.push(Line::from(""));

                for cmd in section.commands.iter().take(5) {
                    lines.push(Line::from(vec![
                        Span::styled("  ", Style::default()),
                        Span::styled(cmd.name, Style::default().fg(Color::Cyan)),
                    ]));
                }
                if section.commands.len() > 5 {
                    lines.push(Line::from(vec![Span::styled(
                        format!("  ... and {} more", section.commands.len() - 5),
                        Style::default().fg(Color::DarkGray),
                    )]));
                }

                lines
            }
            Some(NavItem::Command {
                section_index,
                cmd_index,
            }) => {
                let section = &SECTIONS[*section_index];
                let cmd = &section.commands[*cmd_index];

                vec![
                    Line::from(""),
                    Line::from(vec![
                        Span::styled(
                            format!("[{}] ", section.icon),
                            Style::default().fg(section.color),
                        ),
                        Span::styled(section.name, Style::default().fg(Color::DarkGray)),
                    ]),
                    Line::from(""),
                    Line::from(vec![Span::styled(
                        format!("spn {}", cmd.name),
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    )]),
                    Line::from(""),
                    Line::from(vec![Span::styled(
                        cmd.desc,
                        Style::default().fg(Color::White),
                    )]),
                    Line::from(""),
                    Line::from(vec![Span::styled(
                        "Run with --help for more details",
                        Style::default().fg(Color::DarkGray),
                    )]),
                ]
            }
            None => vec![Line::from("No selection")],
        };

        let details = Paragraph::new(content).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::DarkGray))
                .title(" Details "),
        );

        f.render_widget(details, area);
    }

    /// Render the footer with keybindings.
    fn render_footer(&self, f: &mut Frame<'_>, area: Rect) {
        let hints =
            " j/k: navigate | Enter: expand | e: expand all | c: collapse all | ?: help | q: quit ";
        let footer = Paragraph::new(hints).style(Style::default().fg(Color::DarkGray));
        f.render_widget(footer, area);
    }

    /// Render the help overlay.
    fn render_help_overlay(&self, f: &mut Frame<'_>) {
        let area = centered_rect(50, 60, f.area());
        f.render_widget(Clear, area);

        let help_text = vec![
            Line::from(""),
            Line::from(Span::styled(
                "  Keyboard Shortcuts",
                Style::default().add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from("  j / Down       Move down"),
            Line::from("  k / Up         Move up"),
            Line::from("  Enter / Space  Expand/collapse section"),
            Line::from("  e              Expand all sections"),
            Line::from("  c              Collapse all sections"),
            Line::from("  ?              Toggle this help"),
            Line::from("  q / Esc        Quit"),
            Line::from(""),
            Line::from(Span::styled(
                "  About",
                Style::default().add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(format!("  spn v{}", VERSION)),
            Line::from("  The Agentic AI Toolkit"),
            Line::from(""),
            Line::from("  https://github.com/supernovae-st/supernovae-cli"),
            Line::from(""),
        ];

        let help = Paragraph::new(help_text).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Cyan))
                .title(" Help "),
        );

        f.render_widget(help, area);
    }
}

impl Default for HelpTui {
    fn default() -> Self {
        Self::new()
    }
}

/// Calculate a centered rect given percentage dimensions.
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_width = r.width * percent_x / 100;
    let popup_height = r.height * percent_y / 100;
    let popup_x = (r.width.saturating_sub(popup_width)) / 2;
    let popup_y = (r.height.saturating_sub(popup_height)) / 2;
    Rect::new(r.x + popup_x, r.y + popup_y, popup_width, popup_height)
}

// =============================================================================
// PUBLIC API
// =============================================================================

/// Run the interactive help TUI.
///
/// Returns `Ok(())` on successful exit, or an error if terminal setup fails.
pub fn run() -> io::Result<()> {
    let mut app = HelpTui::new();
    app.run()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_help_tui_new() {
        let tui = HelpTui::new();
        // Should have all sections
        assert_eq!(tui.items.len(), SECTIONS.len());
        // All collapsed initially
        assert!(tui.expanded.iter().all(|&e| !e));
    }

    #[test]
    fn test_sections_data() {
        // Verify all sections have data
        assert!(!SECTIONS.is_empty());
        for section in SECTIONS {
            assert!(!section.name.is_empty());
            assert!(!section.icon.is_empty());
            assert!(!section.commands.is_empty());
        }
    }

    #[test]
    fn test_rebuild_items() {
        let mut tui = HelpTui::new();
        let initial_count = tui.items.len();

        // Expand first section
        tui.expanded[0] = true;
        tui.rebuild_items();

        // Should have more items now
        let expanded_count = tui.items.len();
        assert!(expanded_count > initial_count);

        // Collapse
        tui.expanded[0] = false;
        tui.rebuild_items();
        assert_eq!(tui.items.len(), initial_count);
    }

    #[test]
    fn test_navigation() {
        let mut tui = HelpTui::new();

        // Initial selection
        assert_eq!(tui.selected, 0);

        // Move down
        tui.next();
        assert_eq!(tui.selected, 1);

        // Move up
        tui.prev();
        assert_eq!(tui.selected, 0);

        // Wrap around up
        tui.prev();
        assert_eq!(tui.selected, tui.items.len() - 1);

        // Wrap around down
        tui.next();
        assert_eq!(tui.selected, 0);
    }

    #[test]
    fn test_toggle_expand() {
        let mut tui = HelpTui::new();

        // Select first section and expand
        tui.selected = 0;
        tui.toggle_expand();

        // Should be expanded now
        assert!(tui.expanded[0]);

        // Toggle again
        tui.toggle_expand();
        assert!(!tui.expanded[0]);
    }

    #[test]
    fn test_centered_rect() {
        let outer = Rect::new(0, 0, 100, 50);
        let inner = centered_rect(50, 50, outer);

        assert_eq!(inner.width, 50);
        assert_eq!(inner.height, 25);
        assert_eq!(inner.x, 25);
        assert_eq!(inner.y, 12);
    }
}
