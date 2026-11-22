use focus_core::{Mode, Settings, WidgetPreferences};
use focus_storage::{ActivityHistory, Period, Report, Task};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TimerSnapshot {
    pub revision: u64,
    pub mode: Mode,
    pub running: bool,
    pub started: bool,
    pub remaining_ms: u64,
    pub total_ms: u64,
    pub cycle: u32,
    pub bound_task: Option<String>,
    pub selected_task: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TaskDto {
    pub id: String,
    pub title: String,
    pub estimate: u32,
    pub completed: u32,
    pub done: bool,
    pub deleted: bool,
    pub planned_for: Option<String>,
    pub today_order: i64,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub tag: String,
}
impl From<&Task> for TaskDto {
    fn from(t: &Task) -> Self {
        Self {
            id: t.id.to_string(),
            title: t.title.clone(),
            estimate: t.estimate,
            completed: t.completed,
            done: t.done,
            deleted: t.deleted,
            planned_for: t.planned_for.clone(),
            today_order: t.today_order,
            start_date: t.start_date.clone(),
            end_date: t.end_date.clone(),
            tag: t.tag.clone(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AppSnapshot {
    pub revision: u64,
    pub timer: TimerSnapshot,
    pub tasks: Vec<TaskDto>,
    pub settings: Settings,
    pub widgets: [WidgetPreferences; 3],
    pub today: String,
    pub today_ms: u64,
    pub today_completed: u32,
    pub storage_blocked: bool,
    pub data_location: String,
    pub last_backup: Option<String>,
    pub recovery_message: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ReportQuery {
    #[serde(default)]
    pub period: ReportPeriod,
    #[serde(default)]
    pub offset: i32,
    #[serde(default)]
    pub start: String,
    #[serde(default)]
    pub end: String,
}
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReportPeriod {
    Day,
    #[default]
    Week,
    Month,
}
impl ReportPeriod {
    pub fn storage(self) -> Period {
        match self {
            Self::Day => Period::Day,
            Self::Week => Period::Week,
            Self::Month => Period::Month,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReportDay {
    pub label: String,
    pub active_ms: u64,
    pub completed: u32,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReportTask {
    pub id: Option<String>,
    pub title: String,
    pub deleted: bool,
    pub active_ms: u64,
    pub completed: u32,
    pub done: bool,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub tag: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HourBucketDto {
    pub label: String,
    pub active_ms: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TagReportDto {
    pub tag: String,
    pub active_ms: u64,
    pub completed: u32,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReportDto {
    pub title: String,
    pub days: Vec<ReportDay>,
    pub tasks: Vec<ReportTask>,
    pub hours: Vec<HourBucketDto>,
    pub tags: Vec<TagReportDto>,
    pub active_ms: u64,
    pub completed: u32,
    pub partial_ms: u64,
}
impl From<Report> for ReportDto {
    fn from(r: Report) -> Self {
        Self {
            title: r.title,
            active_ms: r.active_ms,
            completed: r.completed,
            partial_ms: r.partial_ms,
            days: r
                .days
                .into_iter()
                .map(|d| ReportDay {
                    label: d.label,
                    active_ms: d.active_ms,
                    completed: d.completed,
                })
                .collect(),
            tasks: r
                .tasks
                .into_iter()
                .map(|t| ReportTask {
                    id: t.task_id.map(|id| id.to_string()),
                    title: t.title,
                    deleted: t.deleted,
                    active_ms: t.active_ms,
                    completed: t.completed,
                    done: t.done,
                    start_date: t.start_date,
                    end_date: t.end_date,
                    tag: t.tag,
                })
                .collect(),
            hours: r
                .hours
                .into_iter()
                .map(|h| HourBucketDto {
                    label: h.label,
                    active_ms: h.active_ms,
                })
                .collect(),
            tags: r
                .tags
                .into_iter()
                .map(|t| TagReportDto {
                    tag: t.tag,
                    active_ms: t.active_ms,
                    completed: t.completed,
                })
                .collect(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ActivityDayDto {
    pub date: String,
    pub active_ms: u64,
    pub completed: u32,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ActivityHistoryDto {
    pub days: Vec<ActivityDayDto>,
    pub current_streak: u32,
    pub best_streak: u32,
    pub goal_days: u32,
}
impl From<ActivityHistory> for ActivityHistoryDto {
    fn from(h: ActivityHistory) -> Self {
        Self {
            days: h
                .days
                .into_iter()
                .map(|d| ActivityDayDto {
                    date: d.date,
                    active_ms: d.active_ms,
                    completed: d.completed,
                })
                .collect(),
            current_streak: h.current_streak,
            best_streak: h.best_streak,
            goal_days: h.goal_days,
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TimerAction {
    Start,
    Pause,
    Toggle,
    Skip,
    Reset,
    Suspend,
    Resume,
    Checkpoint,
}
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskAction {
    ToggleDone,
    Delete,
    Restore,
    Plan,
    Unplan,
    Up,
    Down,
}

/// The only mutation interface. File paths are supplied by the trusted native dialog host.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum AppCommand {
    Snapshot,
    Timer {
        action: TimerAction,
    },
    ExtendTimer {
        minutes: u32,
    },
    SelectMode {
        mode: Mode,
    },
    SelectTask {
        id: Option<String>,
    },
    StartTask {
        id: String,
    },
    CreateTask {
        title: String,
        estimate: u32,
        today: bool,
        start: bool,
        #[serde(default)]
        start_date: Option<String>,
        #[serde(default)]
        end_date: Option<String>,
        #[serde(default)]
        tag: String,
    },
    EditTask {
        id: String,
        title: String,
        estimate: u32,
        #[serde(default)]
        start_date: Option<String>,
        #[serde(default)]
        end_date: Option<String>,
        #[serde(default)]
        tag: String,
    },
    Task {
        id: String,
        action: TaskAction,
    },
    SaveSettings {
        settings: Settings,
    },
    Widget {
        index: usize,
        preferences: WidgetPreferences,
    },
    MoveWidget {
        index: usize,
        x: i32,
        y: i32,
    },
    Report {
        query: ReportQuery,
    },
    ActivityHistory {
        days: u32,
    },
    SessionDetails {
        id: Option<String>,
        query: ReportQuery,
    },
    Backup {
        path: String,
    },
    Restore {
        path: String,
    },
    Export {
        path: String,
        query: ReportQuery,
    },
    RetrySaving,
    Finish,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum Reply {
    Saved(u64),
    Snapshot(Box<AppSnapshot>),
    Report(ReportDto),
    ActivityHistory(ActivityHistoryDto),
    Details(Vec<String>),
    Created(String),
    File(String),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RuntimeError {
    pub message: String,
    pub accepted: bool,
}
impl From<String> for RuntimeError {
    fn from(message: String) -> Self {
        Self {
            message,
            accepted: false,
        }
    }
}
pub type Result<T> = std::result::Result<T, RuntimeError>;

#[derive(Clone, Debug)]
pub enum RuntimeEvent {
    Changed {
        revision: u64,
        domains: Vec<&'static str>,
    },
    Timer(TimerSnapshot),
    Completed(Mode),
    Settings(Settings),
    Error(RuntimeError),
}
