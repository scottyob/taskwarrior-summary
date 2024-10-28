use std::collections::HashMap;

use color_eyre::Result;

use ansi_to_tui::IntoText;
use ratatui::{
    buffer::Buffer,
    crossterm::event::{self, Event, KeyCode, KeyEventKind},
    layout::{Constraint, Layout, Rect},
    style::{palette::tailwind, Color, Stylize},
    widgets::{Block, Padding, Paragraph, Tabs, Widget},
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
    let app_result = App::default().run(terminal);
    ratatui::restore();
    app_result
}

#[derive(Default)]
struct App {
    state: AppState,
    selected_tab: SelectedTab,
    reports: HashMap<SelectedTab, String>,
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
)]
enum SelectedTab {
    #[default]
    #[strum(to_string = "Due", props(cmd = "project.not:Bethany due"))]
    Due,
    #[strum(
        to_string = "Active",
        props(cmd = "project.not:Bethany active", Color = "false")
    )]
    Tab2,
    #[strum(to_string = "Inbox", props(cmd = "-PROJECT"))]
    Tab3,
}

impl App {
    fn run(mut self, mut terminal: DefaultTerminal) -> Result<()> {
        self.reload_reports();

        while self.state == AppState::Running {
            terminal.draw(|frame| frame.render_widget(&self, frame.area()))?;
            self.handle_events()?;
        }
        Ok(())
    }

    fn handle_events(&mut self) -> std::io::Result<()> {
        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press {
                match key.code {
                    KeyCode::Char('l') | KeyCode::Right => self.next_tab(),
                    KeyCode::Char('h') | KeyCode::Left => self.previous_tab(),
                    KeyCode::Char('q') | KeyCode::Esc => self.quit(),
                    _ => {}
                }
            }
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

    pub fn next_tab(&mut self) {
        self.selected_tab = self.selected_tab.next();
    }

    pub fn previous_tab(&mut self) {
        self.selected_tab = self.selected_tab.previous();
    }

    pub fn quit(&mut self) {
        self.state = AppState::Quitting;
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
}

impl Widget for &App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        use Constraint::{Length, Min};
        let vertical = Layout::vertical([Length(1), Min(0)]);
        let [header_area, inner_area] = vertical.areas(area);

        let horizontal = Layout::horizontal([Min(0)]);
        let [tabs_area] = horizontal.areas(header_area);

        self.render_tabs(tabs_area, buf);

        // Get the output for the tab
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
            let tab_output = self.reports.get(&t).expect("Cmd result expected");

            format!(" {} ({}) ", t, taskwarrior::task_count(tab_output))
                .fg(tailwind::SLATE.c600)
                .bg(Color::default())
        });

        // let titles = SelectedTab::iter().map(SelectedTab::title);
        let highlight_style = (Color::default(), Color::default());
        let selected_tab_index = self.selected_tab as usize;
        Tabs::new(titles)
            .highlight_style(highlight_style)
            .select(selected_tab_index)
            .padding("", "")
            .divider(" ")
            .render(area, buf);
    }
}
