use std::{collections::HashMap, io::stdout, time::Duration};

use color_eyre::Result;

use ansi_to_tui::IntoText;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture, MouseEvent, MouseEventKind},
    execute,
};
use clap::Parser;
use ratatui::{
    buffer::Buffer,
    crossterm::event::{self, Event, KeyCode, KeyEventKind},
    layout::{Constraint, Layout, Position, Rect},
    style::{palette::tailwind, Color, Stylize},
    widgets::{Block, Padding, Paragraph, StatefulWidget, Tabs, Widget},
    DefaultTerminal,
};
use strum::{Display, EnumIter, FromRepr, IntoEnumIterator};

use strum_macros;
// bring the trait into scope
use strum::EnumProperty;

mod taskwarrior;

fn main() -> Result<()> {
    color_eyre::install()?;
    let terminal = ratatui::init();

    // Setup mouse capture events
    execute!(stdout(), EnableMouseCapture)?;

    let app_result = App::default().run(terminal);
    ratatui::restore();
    if let Err(err) = execute!(stdout(), DisableMouseCapture) {
        eprintln!("Error disabling mouse capture: {err}");
    }
    app_result
}

// Use clap to parse arguments and specify possible values
#[derive(Parser)]
struct Cli {
    #[arg(value_enum)]
    tab: SelectedTab,
}


#[derive(Default)]
struct App {
    app_state: AppState,
    selected_tab: SelectedTab,

    reports: HashMap<SelectedTab, String>,
    pub event: Option<MouseEvent>,
}

#[derive(Default, Clone, Copy, PartialEq, Eq)]
enum AppState {
    #[default]
    Running,
    Quitting,
}

#[derive(
    Default,
    Clone,
    Copy,
    Display,
    FromRepr,
    EnumIter,
    strum_macros::EnumProperty,
    PartialEq,
    Eq,
    Hash,
    clap::ValueEnum,
)]
enum SelectedTab {
    #[default]
    #[strum(to_string = "Due", props(cmd = "project.not:Bethany due"))]
    Due,
    #[strum(
        to_string = "Active",
        props(cmd = "project.not:Bethany active", Color = "false")
    )]
    Active,
    #[strum(to_string = "Inbox", props(cmd = "-PROJECT"))]
    Inbox,
}

impl App {
    fn run(mut self, mut terminal: DefaultTerminal) -> Result<()> {
        self.reload_reports();

        let args = Cli::parse();
        
        while self.app_state == AppState::Running {
            self.handle_events()?;
            let mut new_state = args.tab;
            terminal
                .draw(|frame| frame.render_stateful_widget(&self, frame.area(), &mut new_state))?;
            self.selected_tab = new_state;
            self.event = None;
        }
        Ok(())
    }

    fn handle_events(&mut self) -> std::io::Result<()> {
        // Polls in 2 second cycles
        let poll = event::poll(Duration::from_secs(2));
        if poll.is_ok() && poll.unwrap() == false {
            // 10 seconds has passed, idle reload the reports
            self.reload_reports();
            return Ok(())
        }

        match event::read()? {
            Event::Key(key) => {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('l') | KeyCode::Right => self.next_tab(),
                        KeyCode::Char('h') | KeyCode::Left => self.previous_tab(),
                        KeyCode::Char('q') | KeyCode::Esc => self.quit(),
                        _ => {}
                    }
                }
            }
            Event::Mouse(mouse) => {
                if mouse.kind == MouseEventKind::Down(event::MouseButton::Left) {
                    self.event = Some(mouse);
                }
            }
            _ => (),
        }

        Ok(())
    }

    fn reload_reports(&mut self) {
        // Will run each report and store the result
        for tab in SelectedTab::iter() {
            let cmd = tab.get_str("cmd").expect("Enum expected command");
            let color_str = tab.get_str("Color").unwrap_or("true");
            let mut color = true;
            if color_str == "false" {
                color = false;
            }

            let output = taskwarrior::run(color, String::from(cmd).split(' '));
            let output = output.expect("Expected TaskWarrior cmd to have a result");

            self.reports.insert(tab, output);
        }
    }

    pub fn mouse_cord_to_tab(&self, pos: Position) -> Option<SelectedTab> {
        let mut offset = 0;
        for tab in SelectedTab::iter() {
            let report = self.reports.get(&tab).unwrap();
            let width = tab.title(report).len() as u16;
            if pos.x < offset + width {
                return Some(tab);
            }
            offset += width;
        }

        return None;
    }

    pub fn next_tab(&mut self) {
        self.selected_tab = self.selected_tab.next();
    }

    pub fn previous_tab(&mut self) {
        self.selected_tab = self.selected_tab.previous();
    }

    pub fn quit(&mut self) {
        self.app_state = AppState::Quitting;
    }
}

impl SelectedTab {
    /// Get the previous tab, if there is no previous tab return the current tab.
    fn previous(self) -> Self {
        let current_index: usize = self as usize;
        let previous_index = current_index.saturating_sub(1);
        Self::from_repr(previous_index).unwrap_or(self)
    }

    /// Get the next tab, if there is no next tab return the current tab.
    fn next(self) -> Self {
        let current_index = self as usize;
        let next_index = current_index.saturating_add(1);
        Self::from_repr(next_index).unwrap_or(self)
    }

    fn title(self, report: &String) -> String {
        return format!(" {} ({}) ", self, taskwarrior::task_count(report));
    }
}

impl StatefulWidget for &App {
    type State = SelectedTab;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut SelectedTab) {
        use Constraint::{Length, Min};
        let vertical = Layout::vertical([Length(1), Min(0)]);
        let [header_area, inner_area] = vertical.areas(area);
        let horizontal = Layout::horizontal([Min(0)]);
        let [tabs_area] = horizontal.areas(header_area);

        // Render the tabs
        self.render_tabs(tabs_area, buf);

        // Check for mouse events to update the selected tab
        match self.event {
            Some(e) => {
                let pos = Position::new(e.column, e.row);
                if tabs_area.contains(pos) {
                    let clicked_tab = self.mouse_cord_to_tab(pos);
                    if clicked_tab.is_some() {
                        *state = clicked_tab.unwrap();
                    }
                }
            }
            _ => {}
        }

        // Get the main body output for the tab
        let tab_output = self
            .reports
            .get(&self.selected_tab)
            .expect("Cmd result expected");
        let text = tab_output.into_text().unwrap();
        Paragraph::new(text)
            .block(Block::new().padding(Padding::uniform(1)))
            .render(inner_area, buf);
    }
}

impl App {
    fn render_tabs(&self, area: Rect, buf: &mut Buffer) {
        let titles = SelectedTab::iter().map(|t| {
            let tab_output = self.reports.get(&t).expect("Expected report for enum");

            t.title(tab_output)
                .fg(tailwind::SLATE.c600)
                .bg(Color::default())
        });

        // let titles = SelectedTab::iter().map(SelectedTab::title);
        let highlight_style = (Color::default(), Color::default());
        let selected_tab_index = self.selected_tab as usize;
        let tabs = Tabs::new(titles)
            .highlight_style(highlight_style)
            .select(selected_tab_index)
            .padding("", "")
            .divider("");

        tabs.render(area, buf);
    }
}
