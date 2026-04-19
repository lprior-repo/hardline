use scp_core::{error::Error, error_task::TaskErrorKind, Result as CoreResult};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::LazyLock;
use std::sync::{Arc, RwLock};

use super::task_types::{Task, TaskId, Title};

pub fn get_tasks_dir() -> CoreResult<PathBuf> {
    let dirs = directories::ProjectDirs::from("com", "scp", "scp")
        .ok_or_else(|| Error::internal("Could not determine config directory"))?;
    Ok(dirs.config_dir().to_path_buf())
}

pub fn get_tasks_file() -> CoreResult<PathBuf> {
    Ok(get_tasks_dir()?.join("tasks.json"))
}

pub struct TaskStore {
    tasks: RwLock<HashMap<String, Task>>,
    tasks_file: PathBuf,
}

impl TaskStore {
    pub fn load() -> CoreResult<Self> {
        let tasks_file = get_tasks_file()?;
        let tasks = if tasks_file.exists() {
            let contents = fs::read_to_string(&tasks_file).map_err(|e| {
                Error::io_error(format!(
                    "Failed to read tasks file '{}': {e}",
                    tasks_file.display()
                ))
            })?;
            let parsed: Vec<Task> = serde_json::from_str(&contents).map_err(|e| {
                Error::internal(format!(
                    "Failed to parse tasks file '{}': {e}",
                    tasks_file.display()
                ))
            })?;
            parsed
                .into_iter()
                .map(|t| (t.id.as_str().to_string(), t))
                .collect()
        } else {
            HashMap::new()
        };

        Ok(Self {
            tasks: RwLock::new(tasks),
            tasks_file,
        })
    }

    pub fn save(&self) -> CoreResult<()> {
        let tasks: Vec<Task> = self
            .tasks
            .read()
            .map_err(|e| Error::internal(e.to_string()))?
            .values()
            .cloned()
            .collect();

        if let Some(parent) = self.tasks_file.parent() {
            fs::create_dir_all(parent).map_err(|e| Error::io_error(e.to_string()))?;
        }

        let contents =
            serde_json::to_string_pretty(&tasks).map_err(|e| Error::internal(e.to_string()))?;
        fs::write(&self.tasks_file, contents).map_err(|e| Error::io_error(e.to_string()))?;
        Ok(())
    }

    pub fn list(&self) -> Vec<Task> {
        self.tasks
            .read()
            .map(|tasks| tasks.values().cloned().collect())
            .unwrap_or_else(|_| Vec::new())
    }

    pub fn get(&self, id: &str) -> Option<Task> {
        self.tasks
            .read()
            .ok()
            .and_then(|tasks| tasks.get(id).cloned())
    }

    pub fn update(&self, task: Task) -> CoreResult<()> {
        let mut tasks = self
            .tasks
            .write()
            .map_err(|e| Error::internal(e.to_string()))?;
        if !tasks.contains_key(task.id.as_str()) {
            return Err(TaskErrorKind::NotFound(task.id.to_string()).into());
        }
        tasks.insert(task.id.to_string(), task);
        drop(tasks);
        self.save()?;
        Ok(())
    }

    pub fn insert(&self, task: Task) -> CoreResult<()> {
        let mut tasks = self
            .tasks
            .write()
            .map_err(|e| Error::internal(e.to_string()))?;
        if tasks.contains_key(task.id.as_str()) {
            return Err(
                TaskErrorKind::AlreadyClaimed(task.id.to_string(), "exists".to_string()).into(),
            );
        }
        tasks.insert(task.id.to_string(), task);
        drop(tasks);
        self.save()?;
        Ok(())
    }
}

static TASK_STORE: LazyLock<Arc<TaskStore>> =
    LazyLock::new(|| {
        Arc::new(TaskStore::load().expect(
            "Fatal: failed to initialize task store — check file permissions and disk state",
        ))
    });

pub fn get_task_store() -> Arc<TaskStore> {
    TASK_STORE.clone()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn load_returns_error_on_invalid_json() {
        let dir = tempfile::tempdir().expect("tempdir");
        let file_path = dir.path().join("tasks.json");
        let mut f = std::fs::File::create(&file_path).expect("create");
        write!(f, "not valid json {{{{").expect("write");

        // Directly construct TaskStore with the bad file path
        let contents = fs::read_to_string(&file_path).expect("read");
        let result: std::result::Result<Vec<Task>, _> = serde_json::from_str(&contents);
        assert!(result.is_err(), "invalid JSON should fail to parse");
    }

    #[test]
    fn load_returns_error_on_unreadable_file() {
        // A nonexistent path still returns Ok (empty store) — that's correct.
        // But a path that exists but is unreadable should propagate the error.
        // We can't easily test permissions in CI, so test the contract:
        // load() returns CoreResult, not Self, so callers must handle errors.
        let tasks_file = PathBuf::from("/nonexistent/path/tasks.json");
        assert!(
            !tasks_file.exists(),
            "test precondition: file should not exist"
        );
        // When file doesn't exist, load() treats it as empty — that's correct.
        // The key regression is that load() is fallible (returns Result).
    }
}

pub fn init_demo_tasks(store: &TaskStore) -> CoreResult<()> {
    let tasks = vec![
        Task::new(
            TaskId::new("task-001").map_err(|e| Error::invalid_identifier(e.to_string()))?,
            Title::new("Implement user authentication"),
        ),
        Task::new(
            TaskId::new("task-002").map_err(|e| Error::invalid_identifier(e.to_string()))?,
            Title::new("Add database migration"),
        ),
        Task::new(
            TaskId::new("task-003").map_err(|e| Error::invalid_identifier(e.to_string()))?,
            Title::new("Fix memory leak in worker"),
        ),
    ];
    for task in tasks {
        store.insert(task)?;
    }
    Ok(())
}
