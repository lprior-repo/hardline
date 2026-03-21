use scp_core::{error::Error, Result as CoreResult};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::LazyLock;
use std::sync::{Arc, RwLock};

use super::task_types::{Task, TaskId, Title};

pub fn get_tasks_dir() -> CoreResult<PathBuf> {
    let dirs = directories::ProjectDirs::from("com", "scp", "scp")
        .ok_or_else(|| Error::Internal("Could not determine config directory".into()))?;
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
    pub fn load() -> Self {
        let tasks_file = match get_tasks_file() {
            Ok(path) => path,
            Err(e) => {
                eprintln!("Warning: Failed to get tasks file path: {e}. Using default.");
                return Self {
                    tasks: RwLock::new(HashMap::new()),
                    tasks_file: PathBuf::new(),
                };
            }
        };
        let tasks = if tasks_file.exists() {
            match fs::read_to_string(&tasks_file) {
                Ok(contents) => match serde_json::from_str::<Vec<Task>>(&contents) {
                    Ok(tasks) => {
                        let map: HashMap<String, Task> = tasks
                            .into_iter()
                            .map(|t| (t.id.as_str().to_string(), t))
                            .collect();
                        map
                    }
                    Err(e) => {
                        eprintln!("Warning: Failed to parse tasks file: {e}. Starting fresh.");
                        HashMap::new()
                    }
                },
                Err(e) => {
                    eprintln!("Warning: Failed to read tasks file: {e}. Starting fresh.");
                    HashMap::new()
                }
            }
        } else {
            HashMap::new()
        };

        Self {
            tasks: RwLock::new(tasks),
            tasks_file,
        }
    }

    pub fn save(&self) -> CoreResult<()> {
        let tasks: Vec<Task> = self
            .tasks
            .read()
            .map_err(|e| Error::Internal(e.to_string()))?
            .values()
            .cloned()
            .collect();

        if let Some(parent) = self.tasks_file.parent() {
            fs::create_dir_all(parent).map_err(Error::Io)?;
        }

        let contents =
            serde_json::to_string_pretty(&tasks).map_err(|e| Error::Internal(e.to_string()))?;
        fs::write(&self.tasks_file, contents).map_err(Error::Io)?;
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
            .map_err(|e| Error::Internal(e.to_string()))?;
        if !tasks.contains_key(task.id.as_str()) {
            return Err(Error::TaskNotFound(task.id.to_string()));
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
            .map_err(|e| Error::Internal(e.to_string()))?;
        if tasks.contains_key(task.id.as_str()) {
            return Err(Error::TaskAlreadyClaimed(
                task.id.to_string(),
                "exists".to_string(),
            ));
        }
        tasks.insert(task.id.to_string(), task);
        drop(tasks);
        self.save()?;
        Ok(())
    }
}

static TASK_STORE: LazyLock<Arc<TaskStore>> = LazyLock::new(|| Arc::new(TaskStore::load()));

pub fn get_task_store() -> Arc<TaskStore> {
    TASK_STORE.clone()
}

pub fn init_demo_tasks(store: &TaskStore) -> CoreResult<()> {
    let tasks = vec![
        Task::new(
            TaskId::new("task-001").map_err(|e| Error::InvalidTaskId(e.to_string()))?,
            Title::new("Implement user authentication"),
        ),
        Task::new(
            TaskId::new("task-002").map_err(|e| Error::InvalidTaskId(e.to_string()))?,
            Title::new("Add database migration"),
        ),
        Task::new(
            TaskId::new("task-003").map_err(|e| Error::InvalidTaskId(e.to_string()))?,
            Title::new("Fix memory leak in worker"),
        ),
    ];
    for task in tasks {
        store.insert(task)?;
    }
    Ok(())
}
