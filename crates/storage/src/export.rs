use super::*;
use std::io::Write;

impl Store {
    pub(super) fn custom_report(&self, start: &str, end: &str) -> Result<Report> {
        let start = NaiveDate::parse_from_str(start, "%Y-%m-%d")
            .map_err(|_| "Start date must be YYYY-MM-DD")?;
        let end = NaiveDate::parse_from_str(end, "%Y-%m-%d")
            .map_err(|_| "End date must be YYYY-MM-DD")?;
        if end < start || (end - start).num_days() > 366 {
            return Err("Choose an ordered date range of at most 367 days.".into());
        }
        self.report_between(start, end.succ_opt().ok_or("Date overflow")?)
    }
    pub(super) fn session_details(
        &self,
        task_id: Option<i64>,
        period: Period,
        offset: i32,
        start: &str,
        end: &str,
    ) -> Result<Response> {
        let (start, end) = if start.is_empty() && end.is_empty() {
            date_range(Local::now().date_naive(), period, offset)?
        } else {
            let start =
                NaiveDate::parse_from_str(start, "%Y-%m-%d").map_err(|_| "Invalid start date")?;
            let end = NaiveDate::parse_from_str(end, "%Y-%m-%d").map_err(|_| "Invalid end date")?;
            if end < start || (end - start).num_days() > 366 {
                return Err("Choose an ordered range of at most 367 days".into());
            }
            (start, end.succ_opt().ok_or("Date overflow")?)
        };
        let mut boundaries = vec![local_midnight(start)?];
        let mut day = start;
        while day < end {
            day = day.succ_opt().ok_or("Date overflow")?;
            boundaries.push(local_midnight(day)?);
        }
        let begin = boundaries[0];
        let finish = *boundaries.last().ok_or("Empty range")?;
        let sql = format!(
            "SELECT i.session_id,i.start_ms,i.end_ms,i.active_ms,COALESCE(s.completed,0) FROM ({INTERVALS_IN_RANGE}) i LEFT JOIN sessions s ON s.id=i.session_id WHERE i.task_id IS ?4"
        );
        let mut statement = self.db.prepare(&sql).map_err(|e| e.to_string())?;
        let intervals = statement
            .query_map(
                params![
                    begin,
                    finish,
                    finish.saturating_add(MAX_NORMAL_INTERVAL_MS),
                    task_id
                ],
                |r| {
                    Ok((
                        r.get::<_, i64>(0)?,
                        r.get::<_, i64>(1)?,
                        r.get::<_, i64>(2)?,
                        r.get::<_, i64>(3)?.max(0) as u64,
                        r.get::<_, bool>(4)?,
                    ))
                },
            )
            .map_err(|e| e.to_string())?;
        let mut sessions: BTreeMap<i64, (i64, u64, bool)> = BTreeMap::new();
        for interval in intervals {
            let (id, a, b, active, completed) = interval.map_err(|e| e.to_string())?;
            if b <= a {
                continue;
            }
            let mut credited = 0;
            let first = boundaries
                .partition_point(|boundary| *boundary <= a)
                .saturating_sub(1);
            for bucket in boundaries.windows(2).skip(first) {
                if bucket[0] >= b {
                    break;
                }
                let overlap = b.min(bucket[1]).saturating_sub(a.max(bucket[0])).max(0) as u64;
                credited += (u128::from(active) * u128::from(overlap) / (b - a) as u128) as u64;
            }
            let item = sessions.entry(id).or_insert((a, 0, completed));
            item.0 = item.0.min(a);
            item.1 += credited;
        }
        let mut sessions: Vec<_> = sessions.into_values().collect();
        sessions.sort_by_key(|s| std::cmp::Reverse(s.0));
        let truncated = sessions.len() > 500;
        sessions.truncate(500);
        let mut rows = vec![
            "Focus time is clipped to the selected period. Dates use your Windows time zone."
                .into(),
        ];
        if truncated {
            rows.push(
                "Latest 500 sessions shown. Narrow the period to inspect earlier work.".into(),
            );
        }
        for (started, active, completed) in sessions {
            let date = Local
                .timestamp_millis_opt(started)
                .single()
                .map(|t| t.format("%d %b %Y · %H:%M").to_string())
                .unwrap_or_else(|| "Unknown date".into());
            rows.push(format!(
                "{date}   ·   {}m {:02}s   ·   {}",
                active / 60_000,
                (active / 1000) % 60,
                if completed {
                    "Completed session"
                } else {
                    "Partial / in progress"
                }
            ));
        }
        if rows.len() == 1 {
            rows.push("No recorded focus intervals in this period.".into());
        }
        Ok(Response::SessionDetails(rows))
    }
    pub(super) fn export_report(
        &self,
        path: PathBuf,
        period: Period,
        offset: i32,
        start: String,
        end: String,
    ) -> Result<Response> {
        let report = if start.is_empty() && end.is_empty() {
            self.report(period, offset)?
        } else {
            self.custom_report(&start, &end)?
        };
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
            .map_err(|e| e.to_string())?;
        let result = (|| -> std::io::Result<()> {
            // UTF-8 BOM makes non-ASCII task names readable in common spreadsheet apps.
            file.write_all(b"\xef\xbb\xbf")?;
            writeln!(
                file,
                "record,period,current_utc_offset,task_id,title,active_ms,completed,deleted"
            )?;
            let zone = csv_cell(&Local::now().offset().to_string());
            let title = csv_cell(&report.title);
            writeln!(
                file,
                "summary,{title},{zone},,,{},{},",
                report.active_ms, report.completed
            )?;
            for task in report.tasks {
                writeln!(
                    file,
                    "task,{title},{zone},{},{},{},{},{}",
                    task.task_id.map(|id| id.to_string()).unwrap_or_default(),
                    csv_cell(&task.title),
                    task.active_ms,
                    task.completed,
                    task.deleted
                )?;
            }
            for day in report.days {
                writeln!(
                    file,
                    "day,{title},{zone},,{},{},{},",
                    csv_cell(&day.label),
                    day.active_ms,
                    day.completed
                )?;
            }
            file.sync_all()
        })();
        drop(file);
        if let Err(error) = result {
            let _ = std::fs::remove_file(&path);
            return Err(error.to_string());
        }
        Ok(Response::Exported(path))
    }
}

fn csv_cell(value: &str) -> String {
    let trimmed = value.trim_start_matches(|c: char| c.is_whitespace() || c == '\u{feff}');
    let unsafe_prefix =
        trimmed.starts_with(['=', '+', '-', '@']) || value.starts_with(['\t', '\r', '\n']);
    format!(
        "\"{}{}\"",
        if unsafe_prefix { "'" } else { "" },
        value.replace('"', "\"\"")
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn csv_quotes_and_neutralizes_formula_titles() {
        assert_eq!(csv_cell(" 日本語, \"draft\""), "\" 日本語, \"\"draft\"\"\"");
        assert_eq!(csv_cell("  =SUM(A1)"), "\"'  =SUM(A1)\"");
        assert_eq!(csv_cell("\t@call"), "\"'\t@call\"");
    }
}
