use crate::TaskRow;
use focus_storage::Task;
use slint::{Model, VecModel};
use std::{
    cell::{Cell, RefCell},
    collections::HashMap,
    rc::Rc,
};

#[derive(Clone, Copy)]
struct RowIndices {
    all: usize,
    active: Option<usize>,
}

/// Stable models keep list delegates and focus intact when only selection changes.
#[derive(Default)]
pub struct TaskModels {
    pub all: Rc<VecModel<TaskRow>>,
    pub active: Rc<VecModel<TaskRow>>,
    indices: RefCell<HashMap<i64, RowIndices>>,
    selected: Cell<Option<i64>>,
}

impl TaskModels {
    pub fn rebuild(&self, tasks: &[Task], selected: Option<i64>) {
        let mut rows = Vec::with_capacity(tasks.len());
        let mut active = Vec::new();
        let mut indices = self.indices.borrow_mut();
        indices.clear();
        indices.reserve(tasks.len());
        for task in tasks {
            let row = TaskRow {
                id: task.id as i32,
                title: task.title.as_str().into(),
                estimate: task.estimate as i32,
                completed: task.completed as i32,
                done: task.done,
                selected: selected == Some(task.id),
            };
            let active_index = if task.done {
                None
            } else {
                let index = active.len();
                active.push(row.clone());
                Some(index)
            };
            indices.insert(
                task.id,
                RowIndices {
                    all: rows.len(),
                    active: active_index,
                },
            );
            rows.push(row);
        }
        drop(indices);
        self.selected.set(selected);
        self.all.set_vec(rows);
        self.active.set_vec(active);
    }

    pub fn select(&self, selected: Option<i64>) {
        let previous = self.selected.replace(selected);
        if previous == selected {
            return;
        }
        let indices = self.indices.borrow();
        for id in [previous, selected].into_iter().flatten() {
            if let Some(index) = indices.get(&id) {
                let checked = selected == Some(id);
                Self::set_selected(&self.all, index.all, checked);
                if let Some(active) = index.active {
                    Self::set_selected(&self.active, active, checked);
                }
            }
        }
    }

    fn set_selected(model: &VecModel<TaskRow>, index: usize, selected: bool) {
        if let Some(mut row) = model.row_data(index) {
            row.selected = selected;
            model.set_row_data(index, row);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn task(id: i64, done: bool) -> Task {
        Task {
            id,
            title: format!("Task {id} 日本語"),
            estimate: 3,
            completed: 1,
            done,
        }
    }

    fn selected(model: &VecModel<TaskRow>) -> Vec<i32> {
        (0..model.row_count())
            .filter_map(|i| model.row_data(i))
            .filter(|row| row.selected)
            .map(|row| row.id)
            .collect()
    }

    #[test]
    fn selection_updates_both_lists_without_replacing_the_models() {
        let models = TaskModels::default();
        models.rebuild(
            &[task(10, false), task(20, true), task(30, false)],
            Some(10),
        );
        let all = models.all.clone();
        let active = models.active.clone();
        models.select(Some(30));
        assert!(Rc::ptr_eq(&all, &models.all));
        assert!(Rc::ptr_eq(&active, &models.active));
        assert_eq!(selected(&all), [30]);
        assert_eq!(selected(&active), [30]);
        assert_eq!(all.row_data(0).unwrap().title, "Task 10 日本語");
        models.select(Some(30));
        assert_eq!(selected(&all), [30]);
    }

    #[test]
    fn completed_and_missing_selections_clear_the_widget_selection() {
        let models = TaskModels::default();
        models.rebuild(&[task(10, false), task(20, true)], Some(10));
        models.select(Some(20));
        assert_eq!(selected(&models.all), [20]);
        assert!(selected(&models.active).is_empty());
        models.select(Some(99));
        assert!(selected(&models.all).is_empty());
        models.select(None);
        assert!(selected(&models.active).is_empty());
    }

    #[test]
    fn rebuilding_after_deletion_and_completion_refreshes_row_indices() {
        let models = TaskModels::default();
        models.rebuild(
            &[task(10, false), task(20, false), task(30, false)],
            Some(10),
        );
        models.rebuild(&[task(30, false), task(20, true)], Some(10));
        models.select(Some(20));
        assert_eq!(selected(&models.all), [20]);
        assert!(selected(&models.active).is_empty());
        models.select(Some(30));
        assert_eq!(selected(&models.active), [30]);
        models.rebuild(&[], None);
        models.select(Some(30));
        assert_eq!(models.all.row_count(), 0);
    }
}
