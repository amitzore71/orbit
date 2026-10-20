use super::*;

pub(super) fn draft(s: &Settings) -> SettingsDraft {
    SettingsDraft {
        focus: s.focus_minutes as i32,
        short: s.short_minutes as i32,
        long: s.long_minutes as i32,
        interval: s.long_interval as i32,
        auto_focus: s.auto_focus,
        auto_breaks: s.auto_breaks,
        volume: s.volume as i32,
        sound: s.sound as i32,
        notifications: s.notifications,
        motion: s.reduced_motion,
        mica: s.mica,
        startup: s.autostart,
        appearance: s.appearance as i32,
        toggle: s.hotkey_toggle.as_str().into(),
        show: s.hotkey_show.as_str().into(),
        skip: s.hotkey_skip.as_str().into(),
    }
}

impl App {
    pub(super) fn wire_product(&self) {
        self.main.on_begin_capture(|index| {
            with_app(|a| {
                let settings = Settings {
                    hotkey_toggle: String::new(),
                    hotkey_show: String::new(),
                    hotkey_skip: String::new(),
                    ..a.timer.settings.clone()
                };
                let warnings = a.platform.hotkeys(&settings);
                if warnings.is_empty() {
                    a.main.set_recording_shortcut(index);
                } else {
                    a.main.set_settings_error(warnings.join(" ").into());
                }
            })
        });
        self.main
            .on_cancel_capture(|| with_app(|a| a.cancel_capture()));
        self.main.on_capture_shortcut(|index, value| {
            with_app(|a| {
                let mut value = value.to_string();
                if let Some(key) = value.chars().last() {
                    if ('\u{f704}'..='\u{f70f}').contains(&key) {
                        value.pop();
                        value.push_str(&format!("F{}", u32::from(key) - 0xf704 + 1));
                    } else if key == ' ' {
                        value.pop();
                        value.push_str("Space");
                    }
                }
                if platform_win::parse_hotkey(&value).is_ok_and(|key| key.is_some()) {
                    match index {
                        0 => a.main.set_toggle_hotkey(value.into()),
                        1 => a.main.set_show_hotkey(value.into()),
                        _ => a.main.set_skip_hotkey(value.into()),
                    }
                    a.cancel_capture();
                }
            })
        });
        self.main.on_task_history(|id| {
            with_app(|a| {
                a.detail_generation += 1;
                a.submit(
                    Operation::SessionDetails {
                        task_id: if id < 0 { None } else { Some(i64::from(id)) },
                        period: a.period,
                        offset: a.report_offset,
                        start: a.main.get_range_start().to_string(),
                        end: a.main.get_range_end().to_string(),
                    },
                    a.detail_generation,
                );
            })
        });
        self.main.on_filter_tasks(|| {
            with_app(|a| {
                a.task_models
                    .filter(a.main.get_task_view(), a.main.get_task_search().as_str());
                a.main.set_tasks(a.task_models.filtered.clone().into());
            })
        });
        self.main.on_save_and_start(|| {
            with_app(|a| {
                if !a.main.get_task_saving() && !a.storage.is_blocked() {
                    a.start_after_create = true;
                    a.save_task();
                }
            })
        });
        self.main.on_custom_report(|| {
            with_app(|a| {
                a.command(Command::Checkpoint);
                a.request_report();
            })
        });
        self.main.set_data_location(
            platform_win::database_path(&self.data_dir)
                .display()
                .to_string()
                .into(),
        );
        self.calendar_timer
            .start(slint::TimerMode::Repeated, Duration::from_secs(60), || {
                with_app(|a| a.refresh_calendar())
            });
    }
    pub(super) fn cancel_capture(&self) {
        if self.main.get_recording_shortcut() >= 0 {
            self.main.set_recording_shortcut(-1);
            let warnings = self.platform.hotkeys(&self.timer.settings);
            if !warnings.is_empty() {
                self.main.set_settings_error(warnings.join(" ").into());
            }
        }
    }
    pub(super) fn refresh_calendar(&mut self) {
        let date = focus_storage::local_date();
        if self.calendar_date != date {
            self.calendar_date = date;
            self.refresh_tasks();
            self.request_today();
            if self.main.get_page() == PAGE_REPORTS {
                self.request_report();
            }
        }
    }
    pub(super) fn begin_file_dialog(&mut self, kind: u32) {
        if self.main.get_data_busy() || self.storage.is_blocked() {
            return;
        }
        self.command(if kind == 2 {
            Command::Checkpoint
        } else {
            Command::Pause
        });
        self.main.set_data_busy(true);
        platform_win::choose_file(kind, window_handle(self.main.window()), native_handler());
    }
    pub(super) fn file_selected(
        &mut self,
        kind: u32,
        result: platform_win::Result<Option<PathBuf>>,
    ) {
        self.main.set_data_busy(false);
        match result {
            Ok(Some(path)) => {
                if kind == 1 {
                    self.main
                        .set_restore_path(path.display().to_string().into());
                    self.show();
                } else {
                    let operation = if kind == 2 {
                        Operation::ExportReport {
                            path,
                            period: self.period,
                            offset: self.report_offset,
                            start: self.main.get_range_start().to_string(),
                            end: self.main.get_range_end().to_string(),
                        }
                    } else {
                        Operation::Backup(path)
                    };
                    self.main
                        .set_data_busy(self.submit(operation, DATA_REQUEST));
                }
            }
            Ok(None) => {}
            Err(error) => self.message(&format!("File operation: {error}")),
        }
    }
    pub(super) fn apply_restore(&mut self, loaded: focus_storage::Loaded) {
        for index in 0..3 {
            self.destroy_widget(index);
        }
        self.timer = loaded.checkpoint.map_or_else(
            || Timer::new(loaded.settings.clone()),
            |c| Timer::restore(loaded.settings.clone(), c),
        );
        self.tasks = loaded.tasks;
        self.preferences = loaded.widgets;
        self.start_after_create = false;
        self.create_pending = false;
        self.main.set_task_title("".into());
        self.main.set_edit_id(-1);
        self.main.set_undo_id(-1);
        self.main.set_data_busy(false);
        if let Some(path) = loaded.last_backup {
            self.main
                .set_data_status(format!("Recovery backup: {path}").into());
        }
        self.load_settings();
        self.refresh_tasks();
        self.refresh_widget_rows();
        for index in 0..3 {
            if self.preferences[index].enabled {
                self.create_widget(index);
            }
        }
        // Runtime integrations are reconciled to restored preferences, while
        // startup opt-in was retained from this machine by the storage layer.
        let warnings = self.platform.hotkeys(&self.timer.settings);
        self.message(&if warnings.is_empty() {
            "Backup restored. Your previous data is in backups. The timer is paused.".into()
        } else {
            warnings.join(" ")
        });
        self.request_today();
        self.request_report();
        self.render();
        self.reschedule();
    }
}
