use chrono::{DateTime, Utc};
use crossbeam_channel::{Receiver, Sender};
use ratatui::text::Line;
use serde::Deserialize;

#[derive(Default, Clone)]
pub struct StdLine {
    pub timestamp: DateTime<Utc>,
    pub content: String,
}

impl StdLine {
    pub fn new(content: String) -> Self {
        StdLine {
            timestamp: Utc::now(),
            content,
        }
    }

    // Convert StdLine to a string representation
    pub fn to_string(&self, show_timestamp: bool) -> String {
        if show_timestamp {
            return format!("[{}] {}", self.timestamp.to_rfc3339(), self.content);
        }
        self.content.to_string()
    }

    // Convert StdLine to ratatui::widgets::Line
    pub fn to_ratatui_line(&self, show_timestamp: bool) -> Line {
        Line::from(self.to_string(show_timestamp))
    }
}

#[derive(Deserialize, Debug)]
pub struct Cfg {
    pub groups: Vec<GroupCfg>,
}

#[derive(Deserialize, Debug)]
pub struct GroupCfg {
    pub runners: Vec<RunnerCfg>,
}

#[derive(Deserialize, Debug, Clone)]
pub struct RunnerCfg {
    pub name: String,
    #[serde(default = "default_auto_start")]
    pub auto_start: bool,
    #[serde(default = "default_restart_on_finish")]
    pub restart_on_finish: bool,
    pub dir: String,
    pub args: Vec<String>,
}

fn default_auto_start() -> bool {
    true
}

fn default_restart_on_finish() -> bool {
    false
}

#[derive(Default)]
pub enum EventType {
    #[default]
    Restart,
    Stop,
    Finish,
    ApplicationQuit,
}

#[derive(Default)]
pub struct RunnerEvent {
    pub event_type: EventType,
}

pub struct Runner {
    pub name: String,
    pub args: Vec<String>,
    pub lines: Vec<StdLine>,
    // pub horizontal_scroll_state: ScrollbarState,
    // pub horizontal_scroll_size: usize,
    pub vertical_scroll_position: usize,
    pub vertical_scroll_size: usize,
    pub state: RunnerState,
    pub should_restart: bool,

    pub tx: Sender<RunnerEvent>,
    pub rx: Receiver<RunnerEvent>,
}

#[derive(Default)]
pub enum RunnerState {
    #[default]
    Ready,
    Active,
    Error,
    Finish,
}

#[derive(Default)]
pub struct App {
    pub runners: Vec<Runner>,
    pub groups: Vec<Group>,
    pub active_runner: usize,
    pub has_zoomed_runner: bool,
    pub zoomed_runner: usize,
    pub should_exit: bool,
    pub show_timestamps: bool,
    pub show_help: bool,
    pub show_debug: bool,
    pub debug_lines: Vec<StdLine>,
}

#[derive(Default, Debug)]
pub struct Group {
    pub runners: Vec<usize>,
}
