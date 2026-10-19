//! Opt-in stress driver, excluded from the distributed executable.
use super::*;
use serde_json::json;
use std::cell::Cell;
thread_local! { static DRIVER: slint::Timer = slint::Timer::default(); }

/// Capture actual software-rendered windows at two layouts and in high contrast.
/// Diagnostic-only code and PNG support are absent from ordinary release builds.
pub fn audit(output: PathBuf) {
    const SCENARIOS: [(&str, i32, bool, bool); 16] = [
        ("timer", 0, false, false),
        ("tasks", 1, false, false),
        ("reports", 2, false, false),
        ("widgets", 3, false, false),
        ("settings", 4, false, false),
        ("about", 5, false, false),
        ("timer-compact", 0, true, false),
        ("tasks-compact", 1, true, false),
        ("reports-compact", 2, true, false),
        ("widgets-compact", 3, true, false),
        ("settings-compact", 4, true, false),
        ("about-compact", 5, true, false),
        ("timer-notice", 0, true, false),
        ("timer-high-contrast", 0, true, true),
        ("tasks-high-contrast", 1, true, true),
        ("widgets-high-contrast", 3, true, true),
    ];
    std::fs::create_dir_all(&output).unwrap();
    let step = Rc::new(Cell::new(0usize));
    let rows: Rc<RefCell<Vec<serde_json::Value>>> = Default::default();
    DRIVER.with(|driver| {
        driver.start(
            slint::TimerMode::Repeated,
            Duration::from_millis(450),
            move || {
                let index = step.get();
                if index >= SCENARIOS.len() * 2 {
                    DRIVER.with(slint::Timer::stop);
                    with_app(|app| {
                        std::fs::write(
                            output.join("audit.json"),
                            serde_json::to_vec_pretty(&*rows.borrow()).unwrap(),
                        )
                        .unwrap();
                        app.quit();
                    });
                    return;
                }
                // Configure on one iteration, capture after the backend has laid out/painted.
                if index.is_multiple_of(2) {
                    let scenario = index / 2;
                    with_app(|app| {
                        app.main.set_message("".into());
                        let (name, page, compact, high_contrast) = SCENARIOS[scenario];
                        let (width, height) = if compact { (760., 540.) } else { (940., 740.) };
                        // Match the overridden renderer scale, including when Windows'
                        // monitor scale differs from SLINT_SCALE_FACTOR.
                        let scale = app.main.window().scale_factor();
                        app.main.window().set_size(slint::PhysicalSize::new(
                            (width * scale) as u32,
                            (height * scale) as u32,
                        ));
                        app.main.set_page(page);
                        app.sync_layout();
                        if page == 2 {
                            app.request_report();
                        }
                        if name == "timer-notice" {
                            app.main.set_message("An interrupted session was restored paused. Recovery is approximate; unsaved running time was not added to reports.".into());
                        }
                        if high_contrast {
                            let tokens = app.main.global::<Tokens>();
                            tokens.set_high_contrast(true);
                            tokens.set_system_background(slint::Color::from_rgb_u8(0, 0, 0));
                            tokens.set_system_foreground(slint::Color::from_rgb_u8(255, 255, 255));
                            tokens.set_system_highlight(slint::Color::from_rgb_u8(255, 255, 0));
                            tokens.set_system_highlight_text(slint::Color::from_rgb_u8(0, 0, 0));
                        }
                    });
                } else {
                    let scenario = index / 2;
                    with_app(|app| {
                        let name = SCENARIOS[scenario].0;
                        rows.borrow_mut().push(snapshot(
                            app.main.window(),
                            &output.join(format!("{name}.png")),
                        ));
                        if scenario == 5 {
                            for (i, widget) in app.widgets.iter().enumerate() {
                                if let Some(widget) = widget {
                                    rows.borrow_mut().push(snapshot(widget.window(), &output.join(format!("widget-{i}.png"))));
                                }
                            }
                        }
                    });
                }
                step.set(index + 1);
            },
        )
    });
}

fn snapshot(window: &slint::Window, path: &std::path::Path) -> serde_json::Value {
    let pixels = window
        .take_snapshot()
        .expect("capture software-rendered window");
    let file = std::io::BufWriter::new(std::fs::File::create(path).unwrap());
    let mut encoder = png::Encoder::new(file, pixels.width(), pixels.height());
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder
        .write_header()
        .unwrap()
        .write_image_data(pixels.as_bytes())
        .unwrap();
    json!({"image":path.file_name().unwrap().to_string_lossy(), "width":pixels.width(),"height":pixels.height(),"scale":window.scale_factor()})
}

pub fn start(cycles: u32, output: PathBuf) {
    let cycle = Rc::new(Cell::new(0u32));
    let open = Rc::new(Cell::new(false));
    let failures = Rc::new(Cell::new(0u32));
    let checks = Rc::new(Cell::new(0u32));
    type WeakSet = (
        slint::Weak<TimerWidget>,
        slint::Weak<TasksWidget>,
        slint::Weak<MiniWidget>,
    );
    let weak: Rc<RefCell<Option<WeakSet>>> = Default::default();
    DRIVER.with(|timer| timer.start(slint::TimerMode::Repeated, Duration::from_millis(100), move || {
        let current = cycle.get();
        if !open.get() {
            if let Some((a, b, c)) = weak.borrow_mut().take() {
                checks.set(checks.get() + 3);
                failures.set(failures.get() + u32::from(a.upgrade().is_some()) + u32::from(b.upgrade().is_some()) + u32::from(c.upgrade().is_some()));
            }
            if current >= cycles {
                DRIVER.with(|timer| timer.stop());
                with_app(|app| {
                    app.main.set_page(0);
                    app.command(Command::Start);
                    app.control("hide");
                });
                let path = output.clone();
                let result = json!({"phase":"tray-cooldown", "cycles":current, "weakChecks":checks.get(), "retainedWidgets":failures.get()});
                std::fs::write(&path, serde_json::to_vec_pretty(&result).unwrap()).unwrap();
                let failures = failures.clone();
                let checks = checks.clone();
                slint::Timer::single_shot(Duration::from_secs(45), move || {
                    let mut quiet_ticks = 0;
                    with_app(|app| quiet_ticks = app.diagnostic_ticks);
                    let result = json!({"phase":"complete", "cycles":current, "weakChecks":checks.get(), "retainedWidgets":failures.get(), "trayDisplayTicks":quiet_ticks});
                    std::fs::write(&path, serde_json::to_vec_pretty(&result).unwrap()).unwrap();
                    with_app(|app| app.quit());
                });
                return;
            }
            with_app(|app| {
                app.main.show().unwrap(); app.main.window().set_minimized(false);
                app.main.set_page((current % 5) as i32);
                app.command(Command::Start);
                for index in 0..3 { app.preferences[index].enabled = true; app.create_widget(index); }
                app.attach_windows();
                if let [Some(Widget::Timer(a)), Some(Widget::Tasks(b)), Some(Widget::Mini(c))] = &app.widgets {
                    *weak.borrow_mut() = Some((a.as_weak(), b.as_weak(), c.as_weak()));
                }
                for _ in 0..10 { app.request_report(); app.request_today(); }
            });
            open.set(true);
        } else {
            with_app(|app| {
                app.command(Command::Pause);
                for index in 0..3 { app.preferences[index].enabled = false; app.destroy_widget(index); }
                app.control("hide"); app.diagnostic_ticks = 0;
            });
            cycle.set(current + 1); open.set(false);
            std::fs::write(&output, serde_json::to_vec(&json!({"phase":"cycling", "cycles":current+1, "weakChecks":checks.get(), "retainedWidgets":failures.get()})).unwrap()).unwrap();
        }
    }));
}
