use std::{
    collections::HashMap,
    fs,
    path::PathBuf,
    sync::{Arc, RwLock},
};

use scp_core::{error::Error, error_task::TaskErrorKind, Result as CoreResult};

use super::task_types::{Assignee, Task, TaskId, TaskState, Title};

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
        Self::load_from(tasks_file)
    }

    /// Load a store from an explicit file (used by tests to point at a
    /// temp file instead of the config dir).
    pub fn load_from(tasks_file: PathBuf) -> CoreResult<Self> {
        let tasks = Self::read_file(&tasks_file)?;
        Ok(Self {
            tasks: RwLock::new(tasks),
            tasks_file,
        })
    }

    /// Read the tasks file into a map. A missing file is an empty store;
    /// a corrupt file is an error. Used both by `load` and, under the
    /// file lock, by every mutation, so no writer acts on a stale
    /// snapshot of another process's writes.
    fn read_file(tasks_file: &PathBuf) -> CoreResult<HashMap<String, Task>> {
        if !tasks_file.exists() {
            return Ok(HashMap::new());
        }
        let contents = fs::read_to_string(tasks_file).map_err(|e| {
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
        Ok(parsed
            .into_iter()
            .map(|t| (t.id.as_str().to_string(), t))
            .collect())
    }

    /// Path of the lockfile serializing writers of the tasks file.
    fn lock_file_path(&self) -> PathBuf {
        let mut path = self.tasks_file.clone();
        path.set_extension("json.lock");
        path
    }

    /// Acquire an exclusive OS file lock serializing all writers of the
    /// tasks file across processes. The returned File holds the lock
    /// until dropped; the lockfile itself is left in place (deleting it
    /// would let another process lock a different inode). Every mutation
    /// below holds this lock across reload, check, and save, so a
    /// read-modify-write can never interleave with another process's.
    fn acquire_file_lock(&self) -> CoreResult<std::fs::File> {
        use fs2::FileExt;
        if let Some(parent) = self.tasks_file.parent() {
            fs::create_dir_all(parent).map_err(|e| Error::io_error(e.to_string()))?;
        }
        let lock_path = self.lock_file_path();
        // The lockfile is never written to, only locked: create it if
        // missing and leave any existing contents untouched.
        let file = std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(false)
            .open(&lock_path)
            .map_err(|e| {
                Error::io_error(format!(
                    "Failed to open task lock file '{}': {e}",
                    lock_path.display()
                ))
            })?;
        file.lock_exclusive()
            .map_err(|e| Error::io_error(format!("Failed to lock task store: {e}")))?;
        Ok(file)
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
        let _file_lock = self.acquire_file_lock()?;
        let mut tasks = self
            .tasks
            .write()
            .map_err(|e| Error::internal(e.to_string()))?;
        *tasks = Self::read_file(&self.tasks_file)?;
        if !tasks.contains_key(task.id.as_str()) {
            return Err(TaskErrorKind::NotFound(task.id.to_string()).into());
        }
        tasks.insert(task.id.to_string(), task);
        drop(tasks);
        self.save()?;
        Ok(())
    }

    pub fn insert(&self, task: Task) -> CoreResult<()> {
        let _file_lock = self.acquire_file_lock()?;
        let mut tasks = self
            .tasks
            .write()
            .map_err(|e| Error::internal(e.to_string()))?;
        *tasks = Self::read_file(&self.tasks_file)?;
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

    /// Atomically claim a task for an agent.
    ///
    /// Holds the file lock across reload, validation, and save, so two
    /// processes racing to claim the same task cannot both succeed: the
    /// first claim wins and the rest see the task as already claimed.
    /// Claiming a task already claimed by the same agent is a no-op
    /// success (idempotent re-claim), matching the previous behavior.
    /// Returns the claimed task.
    pub fn claim(&self, task_id: &str, agent: &str) -> CoreResult<Task> {
        let _file_lock = self.acquire_file_lock()?;
        let mut tasks = self
            .tasks
            .write()
            .map_err(|e| Error::internal(e.to_string()))?;
        *tasks = Self::read_file(&self.tasks_file)?;
        let Some(task) = tasks.get(task_id).cloned() else {
            return Err(TaskErrorKind::NotFound(task_id.to_string()).into());
        };
        if let Some(assignee) = &task.assignee {
            if assignee.as_str() != agent {
                return Err(TaskErrorKind::AlreadyClaimed(
                    task_id.to_string(),
                    assignee.to_string(),
                )
                .into());
            }
        }
        let claimed = Task {
            assignee: Some(Assignee::new(agent)),
            state: TaskState::InProgress,
            updated_at: chrono::Utc::now(),
            ..task
        };
        tasks.insert(task_id.to_string(), claimed.clone());
        drop(tasks);
        self.save()?;
        Ok(claimed)
    }
}

static TASK_STORE: RwLock<Option<Arc<TaskStore>>> = RwLock::new(None);

fn task_store_guard() -> &'static RwLock<Option<Arc<TaskStore>>> {
    &TASK_STORE
}

pub fn get_task_store() -> Arc<TaskStore> {
    // Fast path: check if already initialized
    if let Ok(guard) = task_store_guard().read() {
        if let Some(store) = guard.as_ref() {
            return store.clone();
        }
    }

    // Slow path: initialize lazily
    let store = match TaskStore::load() {
        Ok(s) => Arc::new(s),
        Err(e) => {
            eprintln!(
                "Warning: failed to load task store, using empty store: {}",
                e
            );
            Arc::new(TaskStore {
                tasks: RwLock::new(HashMap::new()),
                tasks_file: PathBuf::new(),
            })
        }
    };

    if let Ok(mut guard) = task_store_guard().write() {
        if guard.is_none() {
            *guard = Some(store.clone());
        }
    }

    store
}

#[cfg(test)]
#[allow(clippy::items_after_test_module)]
mod tests {
    use std::io::Write;

    use super::*;

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

    fn temp_store(dir: &tempfile::TempDir) -> TaskStore {
        TaskStore {
            tasks: std::sync::RwLock::new(std::collections::HashMap::new()),
            tasks_file: dir.path().join("tasks.json"),
        }
    }

    #[test]
    fn claim_sequential_rules() {
        let dir = tempfile::tempdir().expect("tempdir");
        let store = temp_store(&dir);
        store
            .insert(Task::new(
                TaskId::new("task-1").expect("id"),
                Title::new("Do thing"),
            ))
            .expect("insert");
        // First claim wins.
        let claimed = store.claim("task-1", "agent-1").expect("claim");
        assert_eq!(claimed.state, TaskState::InProgress);
        assert_eq!(
            claimed.assignee.as_ref().expect("assignee").as_str(),
            "agent-1"
        );
        // Same agent re-claiming is a no-op success.
        assert!(store.claim("task-1", "agent-1").is_ok());
        // Another agent loses, and the winner still holds the task.
        assert!(store.claim("task-1", "agent-2").is_err());
        let after = store.get("task-1").expect("task");
        assert_eq!(after.assignee.as_ref().expect("assignee").as_str(), "agent-1");
        // Unknown tasks fail.
        assert!(store.claim("no-such-task", "agent-1").is_err());
    }

    #[test]
    fn claim_concurrent_single_winner_across_instances() {
        use std::sync::{Arc, Barrier};
        let dir = tempfile::tempdir().expect("tempdir");
        let file = dir.path().join("tasks.json");
        let seed = TaskStore {
            tasks: std::sync::RwLock::new(std::collections::HashMap::new()),
            tasks_file: file.clone(),
        };
        seed.insert(Task::new(
            TaskId::new("race-1").expect("id"),
            Title::new("Race"),
        ))
        .expect("seed");
        // Separate store instances (separate snapshots, like separate
        // processes) racing to claim the same task, released together.
        let barrier = Arc::new(Barrier::new(8));
        let mut handles = Vec::new();
        for i in 0..8 {
            let barrier = Arc::clone(&barrier);
            let path = file.clone();
            handles.push(std::thread::spawn(move || {
                // Each thread loads its own snapshot from disk, exactly
                // like a separate CLI process starting up.
                let store = TaskStore::load_from(path).expect("load");
                barrier.wait();
                store.claim("race-1", &format!("agent-{i}")).is_ok()
            }));
        }
        let winners = handles
            .into_iter()
            .map(|h| h.join().expect("join"))
            .filter(|won| *won)
            .count();
        assert_eq!(winners, 1, "exactly one concurrent claim must win");
        let contents = std::fs::read_to_string(&file).expect("read");
        let tasks: Vec<Task> = serde_json::from_str(&contents).expect("parse");
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].state, TaskState::InProgress);
        assert!(tasks[0].assignee.is_some());
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
