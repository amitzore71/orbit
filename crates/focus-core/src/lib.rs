//! Deterministic timer. The host supplies awake and wall clocks; no UI or OS code lives here.
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Mode {
    #[default]
    Focus,
    ShortBreak,
    LongBreak,
}
impl Mode {
    pub fn label(self) -> &'static str {
        match self {
            Self::Focus => "Focus",
            Self::ShortBreak => "Short break",
            Self::LongBreak => "Long break",
        }
    }
    pub fn index(self) -> i32 {
        match self {
            Self::Focus => 0,
            Self::ShortBreak => 1,
            Self::LongBreak => 2,
        }
    }
    pub fn from_index(i: i32) -> Self {
        match i {
            1 => Self::ShortBreak,
            2 => Self::LongBreak,
            _ => Self::Focus,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Accent {
    #[default]
    Coral,
    Sage,
    Amber,
    SlateBlue,
}
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Density {
    #[default]
    Comfortable,
    Compact,
}
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MotionLevel {
    #[default]
    Standard,
    Reduced,
    Off,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct Settings {
    pub focus_minutes: u32,
    pub short_minutes: u32,
    pub long_minutes: u32,
    pub long_interval: u32,
    pub auto_breaks: bool,
    pub auto_focus: bool,
    pub volume: u32,
    pub sound: u32,
    pub notifications: bool,
    pub reduced_motion: bool,
    pub mica: bool,
    pub autostart: bool,
    pub hotkey_toggle: String,
    pub hotkey_show: String,
    pub hotkey_skip: String,
    /// 0 follows Windows, 1 is light, 2 is dark.
    pub appearance: u32,
    pub accent: Accent,
    pub density: Density,
    /// None preserves the reduced_motion preference in pre-upgrade backups.
    pub motion_level: Option<MotionLevel>,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            focus_minutes: 25,
            short_minutes: 5,
            long_minutes: 15,
            long_interval: 4,
            auto_breaks: false,
            auto_focus: false,
            volume: 65,
            sound: 0,
            notifications: true,
            reduced_motion: false,
            mica: false,
            autostart: false,
            hotkey_toggle: "Ctrl+Alt+P".into(),
            hotkey_show: "Ctrl+Alt+O".into(),
            hotkey_skip: String::new(),
            appearance: 0,
            accent: Accent::Coral,
            density: Density::Comfortable,
            motion_level: None,
        }
    }
}
impl Settings {
    pub fn effective_motion(&self) -> MotionLevel {
        self.motion_level.unwrap_or(if self.reduced_motion {
            MotionLevel::Reduced
        } else {
            MotionLevel::Standard
        })
    }
    pub fn normalize_appearance(&mut self) {
        let motion = self.effective_motion();
        self.motion_level = Some(motion);
        self.reduced_motion = motion != MotionLevel::Standard;
    }
    pub fn validate(&self) -> Result<(), String> {
        if !(1..=180).contains(&self.focus_minutes)
            || !(1..=180).contains(&self.short_minutes)
            || !(1..=180).contains(&self.long_minutes)
        {
            return Err("Durations must be 1–180 minutes.".into());
        }
        if !(1..=12).contains(&self.long_interval) {
            return Err("Long break interval must be 1–12.".into());
        }
        if self.volume > 100 || self.sound > 2 || self.appearance > 2 {
            return Err("Invalid alarm settings.".into());
        }
        if [&self.hotkey_toggle, &self.hotkey_show, &self.hotkey_skip]
            .iter()
            .any(|key| key.len() > 128)
        {
            return Err("Shortcut text is too long.".into());
        }
        Ok(())
    }
    pub fn duration_ms(&self, mode: Mode) -> u64 {
        u64::from(match mode {
            Mode::Focus => self.focus_minutes,
            Mode::ShortBreak => self.short_minutes,
            Mode::LongBreak => self.long_minutes,
        }) * 60_000
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct TimePoint {
    pub awake_ms: u64,
    pub utc_ms: i64,
}
pub trait Clock {
    fn now(&self) -> TimePoint;
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Checkpoint {
    pub mode: Mode,
    pub remaining_ms: u64,
    pub total_ms: u64,
    pub running: bool,
    pub cycle: u32,
    pub session_id: u64,
    pub next_session_id: u64,
    pub bound_task: Option<i64>,
    pub selected_task: Option<i64>,
    pub started: bool,
    pub saved_utc_ms: i64,
}

#[derive(Clone, Debug)]
pub struct Snapshot {
    pub mode: Mode,
    pub running: bool,
    pub remaining_ms: u64,
    pub total_ms: u64,
    pub cycle: u32,
    pub task: Option<i64>,
    pub started: bool,
}
impl Snapshot {
    pub fn progress(&self) -> f32 {
        1.0 - self.remaining_ms as f32 / self.total_ms.max(1) as f32
    }
}

#[derive(Clone, Debug)]
pub enum Effect {
    FocusInterval {
        session_id: u64,
        task_id: Option<i64>,
        start_utc_ms: i64,
        end_utc_ms: i64,
        active_ms: u64,
    },
    SessionEnded {
        session_id: u64,
        task_id: Option<i64>,
        completed: bool,
        ended_utc_ms: i64,
    },
    Completed(Mode),
}
#[derive(Clone, Copy, Debug)]
pub enum Command {
    Start,
    Pause,
    Toggle,
    Skip,
    Reset,
    SelectMode(Mode),
    SelectTask(Option<i64>),
    StartTask(i64),
    Suspend,
    Resume,
    Checkpoint,
    Tick,
}

#[derive(Clone)]
pub struct Timer {
    pub settings: Settings,
    state: Checkpoint,
    deadline_ms: Option<u64>,
    segment: Option<TimePoint>,
    suspended: bool,
}
impl Timer {
    pub fn new(settings: Settings) -> Self {
        let duration = settings.duration_ms(Mode::Focus);
        Self {
            settings,
            state: Checkpoint {
                mode: Mode::Focus,
                remaining_ms: duration,
                total_ms: duration,
                running: false,
                cycle: 0,
                session_id: 0,
                next_session_id: 1,
                bound_task: None,
                selected_task: None,
                started: false,
                saved_utc_ms: 0,
            },
            deadline_ms: None,
            segment: None,
            suspended: false,
        }
    }
    /// Restarts never infer focus time from wall time. An interrupted run is restored paused.
    pub fn restore(settings: Settings, mut state: Checkpoint) -> Self {
        state.running = false;
        if !state.started {
            state.total_ms = settings.duration_ms(state.mode);
            state.remaining_ms = state.total_ms;
        }
        state.total_ms = state.total_ms.max(1);
        state.remaining_ms = state.remaining_ms.min(state.total_ms);
        state.next_session_id = state
            .next_session_id
            .max(state.session_id.saturating_add(1))
            .max(1);
        Self {
            settings,
            state,
            deadline_ms: None,
            segment: None,
            suspended: false,
        }
    }
    pub fn snapshot(&self, now: TimePoint) -> Snapshot {
        Snapshot {
            mode: self.state.mode,
            running: self.state.running,
            remaining_ms: self.remaining(now),
            total_ms: self.state.total_ms,
            cycle: self.state.cycle,
            task: if self.state.started {
                self.state.bound_task
            } else {
                self.state.selected_task
            },
            started: self.state.started,
        }
    }
    pub fn checkpoint(&self, now: TimePoint) -> Checkpoint {
        Checkpoint {
            remaining_ms: self.remaining(now),
            saved_utc_ms: now.utc_ms,
            ..self.state.clone()
        }
    }
    fn remaining(&self, now: TimePoint) -> u64 {
        self.deadline_ms
            .map_or(self.state.remaining_ms, |d| d.saturating_sub(now.awake_ms))
    }
    fn close_segment(&mut self, end: TimePoint, effects: &mut Vec<Effect>) {
        if let Some(start) = self.segment.take() {
            let active_ms = end.awake_ms.saturating_sub(start.awake_ms);
            if self.state.mode == Mode::Focus && active_ms > 0 {
                // Normalize a wall-clock adjustment without fabricating additional focus time.
                let wall_end = start
                    .utc_ms
                    .saturating_add(active_ms.min(i64::MAX as u64) as i64);
                effects.push(Effect::FocusInterval {
                    session_id: self.state.session_id,
                    task_id: self.state.bound_task,
                    start_utc_ms: start.utc_ms,
                    end_utc_ms: wall_end,
                    active_ms,
                });
            }
        }
    }
    fn start(&mut self, now: TimePoint) {
        if self.state.running || self.suspended {
            return;
        }
        if !self.state.started {
            self.state.total_ms = self.settings.duration_ms(self.state.mode);
            self.state.remaining_ms = self.state.total_ms;
            self.state.session_id = self.state.next_session_id;
            self.state.next_session_id += 1;
            self.state.bound_task = self.state.selected_task;
            self.state.started = true;
        }
        self.state.running = true;
        self.deadline_ms = Some(now.awake_ms.saturating_add(self.state.remaining_ms));
        self.segment = Some(now);
    }
    fn load_mode(&mut self, mode: Mode) {
        self.state.mode = mode;
        self.state.total_ms = self.settings.duration_ms(mode);
        self.state.remaining_ms = self.state.total_ms;
        self.state.running = false;
        self.state.started = false;
        self.state.bound_task = None;
        self.deadline_ms = None;
    }
    fn finish(&mut self, completed: bool, now: TimePoint, effects: &mut Vec<Effect>) {
        self.close_segment(now, effects);
        if self.state.started && self.state.mode == Mode::Focus {
            effects.push(Effect::SessionEnded {
                session_id: self.state.session_id,
                task_id: self.state.bound_task,
                completed,
                ended_utc_ms: now.utc_ms,
            });
        }
    }
    fn next_mode(&self) -> Mode {
        if self.state.mode != Mode::Focus {
            Mode::Focus
        } else if self.state.cycle > 0
            && self
                .state
                .cycle
                .is_multiple_of(self.settings.long_interval.max(1))
        {
            Mode::LongBreak
        } else {
            Mode::ShortBreak
        }
    }
    pub fn dispatch(&mut self, command: Command, now: TimePoint) -> Vec<Effect> {
        let mut effects = Vec::new();
        let mut expired = false;
        if let Some(deadline) = self.deadline_ms.filter(|d| now.awake_ms >= *d) {
            expired = true;
            let late = now.awake_ms - deadline;
            let end = TimePoint {
                awake_ms: deadline,
                utc_ms: now.utc_ms.saturating_sub(late.min(i64::MAX as u64) as i64),
            };
            let completed_mode = self.state.mode;
            self.finish(true, end, &mut effects);
            if completed_mode == Mode::Focus {
                self.state.cycle = self.state.cycle.saturating_add(1);
            }
            effects.push(Effect::Completed(completed_mode));
            let next = self.next_mode();
            self.load_mode(next);
            // Start at reconciliation time. Never replay sessions missed by a delayed callback.
            if (next == Mode::Focus && self.settings.auto_focus)
                || (next != Mode::Focus && self.settings.auto_breaks)
            {
                self.start(now);
            }
        }
        match command {
            Command::Start => self.start(now),
            Command::Toggle if !expired => {
                if self.state.running {
                    self.pause(now, &mut effects);
                } else {
                    self.start(now);
                }
            }
            Command::Pause => self.pause(now, &mut effects),
            Command::Skip if !expired => {
                self.finish(false, now, &mut effects);
                let next = self.next_mode();
                self.load_mode(next);
            }
            Command::Reset => {
                self.finish(false, now, &mut effects);
                self.load_mode(self.state.mode);
            }
            Command::SelectMode(mode) => {
                self.finish(false, now, &mut effects);
                self.load_mode(mode);
            }
            Command::SelectTask(task) => self.state.selected_task = task,
            Command::StartTask(task) => {
                self.state.selected_task = Some(task);
                if !self.state.started && self.state.mode == Mode::Focus {
                    self.start(now);
                }
            }
            Command::Suspend => {
                self.close_segment(now, &mut effects);
                self.suspended = true;
            }
            Command::Resume => {
                self.suspended = false;
                if self.state.running && self.segment.is_none() {
                    self.segment = Some(now);
                }
            }
            Command::Checkpoint => {
                self.close_segment(now, &mut effects);
                if self.state.running && !self.suspended {
                    self.segment = Some(now);
                }
            }
            _ => {}
        }
        effects
    }
    fn pause(&mut self, now: TimePoint, effects: &mut Vec<Effect>) {
        self.close_segment(now, effects);
        self.state.remaining_ms = self.remaining(now);
        self.state.running = false;
        self.deadline_ms = None;
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct WidgetPreferences {
    pub enabled: bool,
    pub x: Option<i32>,
    pub y: Option<i32>,
    pub layer: i32,
    pub click_through: bool,
}
impl Default for WidgetPreferences {
    fn default() -> Self {
        Self {
            enabled: false,
            x: None,
            y: None,
            layer: 1,
            click_through: false,
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn start_task_is_atomic_and_never_rebinds_a_started_session() {
        let mut timer = Timer::new(Settings::default());
        timer.dispatch(Command::StartTask(10), t(0, 0));
        assert!(timer.snapshot(t(0, 0)).running);
        timer.dispatch(Command::Pause, t(1000, 1000));
        timer.dispatch(Command::StartTask(20), t(2000, 2000));
        let saved = timer.checkpoint(t(2000, 2000));
        assert_eq!(saved.bound_task, Some(10));
        assert_eq!(saved.selected_task, Some(20));
        assert!(!saved.running);
        timer.dispatch(Command::Reset, t(2000, 2000));
        timer.dispatch(Command::StartTask(20), t(2000, 2000));
        assert_eq!(timer.snapshot(t(2000, 2000)).task, Some(20));
    }

    #[test]
    fn periodic_checkpoint_splits_segments_without_inventing_crash_time() {
        let mut timer = Timer::new(Settings::default());
        timer.dispatch(Command::Start, t(0, 0));
        let effects = timer.dispatch(Command::Checkpoint, t(60_000, 60_000));
        assert!(matches!(
            effects.as_slice(),
            [Effect::FocusInterval {
                active_ms: 60_000,
                ..
            }]
        ));
        let saved = timer.checkpoint(t(60_000, 60_000));
        let mut recovered = Timer::restore(timer.settings.clone(), saved);
        assert!(!recovered.snapshot(t(99_000, 999_000)).running);
        assert!(
            recovered
                .dispatch(Command::Checkpoint, t(99_000, 999_000))
                .is_empty()
        );
        assert_eq!(
            recovered.snapshot(t(99_000, 999_000)).remaining_ms,
            24 * 60_000
        );
    }
    use super::*;
    #[test]
    fn randomized_commands_conserve_active_time_and_complete_each_session_once() {
        for seed in 1..=8u64 {
            let mut random = seed;
            let mut timer = timer();
            let mut now = t(0, 1_700_000_000_000);
            let mut suspended = false;
            let mut expected = 0u64;
            let mut credited = 0u64;
            let mut completions = std::collections::HashSet::new();
            for _ in 0..100_000 {
                random = random.wrapping_mul(6364136223846793005).wrapping_add(1);
                let before = timer.snapshot(now);
                let delta = if suspended {
                    0
                } else {
                    (random >> 24) % 100_000
                };
                if before.running && before.mode == Mode::Focus && !suspended {
                    expected += delta.min(before.remaining_ms);
                }
                now.awake_ms += delta;
                now.utc_ms += delta as i64 + if random & 511 == 0 { -3_600_000 } else { 0 };
                let command = match (random >> 32) % 12 {
                    0 => Command::Start,
                    1 => Command::Pause,
                    2 => Command::Toggle,
                    3 => Command::Skip,
                    4 => Command::Reset,
                    5 => Command::SelectMode(Mode::from_index((random % 3) as i32)),
                    6 => {
                        suspended = true;
                        Command::Suspend
                    }
                    7 => {
                        suspended = false;
                        now.utc_ms += 3_600_000;
                        Command::Resume
                    }
                    8 => Command::Checkpoint,
                    9 => Command::SelectTask(Some((random % 10) as i64)),
                    _ => Command::Tick,
                };
                for effect in timer.dispatch(command, now) {
                    match effect {
                        Effect::FocusInterval {
                            active_ms,
                            start_utc_ms,
                            end_utc_ms,
                            ..
                        } => {
                            assert_eq!(end_utc_ms - start_utc_ms, active_ms as i64);
                            credited += active_ms;
                        }
                        Effect::SessionEnded {
                            session_id,
                            completed: true,
                            ..
                        } => {
                            assert!(completions.insert(session_id));
                        }
                        _ => {}
                    }
                }
                let snapshot = timer.snapshot(now);
                assert!(snapshot.remaining_ms <= snapshot.total_ms);
                assert!((0.0..=1.0).contains(&snapshot.progress()));
                assert!(credited <= expected, "seed {seed}: {credited} > {expected}");
            }
            for effect in timer.dispatch(Command::Pause, now) {
                if let Effect::FocusInterval { active_ms, .. } = effect {
                    credited += active_ms;
                }
            }
            assert_eq!(credited, expected, "seed {seed}");
        }
    }
    fn t(awake: u64, wall: i64) -> TimePoint {
        TimePoint {
            awake_ms: awake,
            utc_ms: wall,
        }
    }
    fn timer() -> Timer {
        Timer::new(Settings {
            focus_minutes: 1,
            short_minutes: 1,
            ..Settings::default()
        })
    }
    #[test]
    fn deadline_does_not_depend_on_ticks() {
        let mut x = timer();
        x.dispatch(Command::Start, t(100, 0));
        assert_eq!(x.snapshot(t(30_100, 30_000)).remaining_ms, 30_000);
        let effects = x.dispatch(Command::Tick, t(90_100, 90_000));
        assert_eq!(
            effects
                .iter()
                .filter(|e| matches!(e, Effect::Completed(_)))
                .count(),
            1
        );
        assert_eq!(x.snapshot(t(90_100, 90_000)).mode, Mode::ShortBreak);
        assert!(x.dispatch(Command::Tick, t(90_101, 90_001)).is_empty());
    }
    #[test]
    fn pause_preserves_remaining_and_task_binding() {
        let mut x = timer();
        x.dispatch(Command::SelectTask(Some(1)), t(0, 0));
        x.dispatch(Command::Start, t(0, 0));
        x.dispatch(Command::SelectTask(Some(2)), t(1000, 1000));
        let e = x.dispatch(Command::Pause, t(10_000, 10_000));
        assert!(matches!(
            e[0],
            Effect::FocusInterval {
                task_id: Some(1),
                active_ms: 10_000,
                ..
            }
        ));
        x.dispatch(Command::Start, t(100_000, 100_000));
        assert_eq!(x.snapshot(t(101_000, 101_000)).remaining_ms, 49_000);
    }
    #[test]
    fn sleep_never_counts_as_focus() {
        let mut x = timer();
        x.dispatch(Command::Start, t(0, 0));
        let a = x.dispatch(Command::Suspend, t(10_000, 10_000));
        x.dispatch(Command::Resume, t(10_000, 3_610_000));
        assert_eq!(x.snapshot(t(10_000, 3_610_000)).remaining_ms, 50_000);
        let b = x.dispatch(Command::Pause, t(20_000, 3_620_000));
        assert!(matches!(
            a[0],
            Effect::FocusInterval {
                active_ms: 10_000,
                ..
            }
        ));
        assert!(matches!(
            b[0],
            Effect::FocusInterval {
                active_ms: 10_000,
                start_utc_ms: 3_610_000,
                ..
            }
        ));
    }
    #[test]
    fn skip_and_reset_do_not_complete_pomodoros() {
        let mut x = timer();
        x.dispatch(Command::Start, t(0, 0));
        let e = x.dispatch(Command::Skip, t(500, 500));
        assert!(!e.iter().any(|e| matches!(e, Effect::Completed(_))));
        assert_eq!(x.snapshot(t(500, 500)).cycle, 0);
    }
    #[test]
    fn interval_uses_awake_time_despite_wall_clock_jump() {
        let mut x = timer();
        x.dispatch(Command::Start, t(0, 1_000_000));
        let e = x.dispatch(Command::Pause, t(1000, 0));
        assert!(matches!(
            e[0],
            Effect::FocusInterval {
                start_utc_ms: 1_000_000,
                end_utc_ms: 1_001_000,
                active_ms: 1000,
                ..
            }
        ));
    }
    #[test]
    fn restore_pauses_and_does_not_credit_absence() {
        let mut x = timer();
        x.dispatch(Command::Start, t(0, 0));
        let cp = x.checkpoint(t(5000, 5000));
        let restored = Timer::restore(x.settings.clone(), cp);
        let s = restored.snapshot(t(9_000_000, 9_000_000));
        assert!(!s.running);
        assert_eq!(s.remaining_ms, 55_000);
    }
    #[test]
    fn long_break_and_duration_changes_apply_next_session() {
        let mut x = timer();
        x.settings.long_interval = 1;
        x.dispatch(Command::Start, t(0, 0));
        x.settings.focus_minutes = 2;
        assert_eq!(x.snapshot(t(1000, 1000)).total_ms, 60_000);
        x.dispatch(Command::Tick, t(60_000, 60_000));
        assert_eq!(x.snapshot(t(60_000, 60_000)).mode, Mode::LongBreak);
        x.dispatch(Command::SelectMode(Mode::Focus), t(60_000, 60_000));
        assert_eq!(x.snapshot(t(60_000, 60_000)).total_ms, 120_000);
    }
    #[test]
    fn delayed_autostart_begins_now_and_toggle_cannot_skip_completion() {
        let mut x = timer();
        x.settings.auto_breaks = true;
        x.dispatch(Command::Start, t(0, 0));
        x.dispatch(Command::Toggle, t(3_600_000, 3_600_000));
        let s = x.snapshot(t(3_600_000, 3_600_000));
        assert!(s.running);
        assert_eq!(s.remaining_ms, 60_000);
        assert_eq!(s.cycle, 1);
    }
}
