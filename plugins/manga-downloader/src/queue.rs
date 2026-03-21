use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Status of a download task.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DownloadStatus {
    Pending,
    Downloading,
    Completed,
    Failed,
    Paused,
}

/// A single download task (one chapter).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadTask {
    pub manga_title: String,
    pub chapter_title: String,
    pub chapter_id: String,
    pub output_dir: PathBuf,
    pub status: DownloadStatus,
    pub pages_total: usize,
    pub pages_downloaded: usize,
    pub error: Option<String>,
}

impl DownloadTask {
    pub fn new(
        manga_title: impl Into<String>,
        chapter_title: impl Into<String>,
        chapter_id: impl Into<String>,
        output_dir: PathBuf,
        pages_total: usize,
    ) -> Self {
        Self {
            manga_title: manga_title.into(),
            chapter_title: chapter_title.into(),
            chapter_id: chapter_id.into(),
            output_dir,
            status: DownloadStatus::Pending,
            pages_total,
            pages_downloaded: 0,
            error: None,
        }
    }

    pub fn progress(&self) -> f32 {
        if self.pages_total == 0 {
            return 0.0;
        }
        self.pages_downloaded as f32 / self.pages_total as f32
    }

    pub fn is_complete(&self) -> bool {
        self.status == DownloadStatus::Completed
    }

    pub fn mark_page_done(&mut self) {
        self.pages_downloaded += 1;
        if self.pages_downloaded >= self.pages_total {
            self.status = DownloadStatus::Completed;
        }
    }

    pub fn mark_failed(&mut self, error: String) {
        self.status = DownloadStatus::Failed;
        self.error = Some(error);
    }

    /// Directory where pages are saved: output_dir/manga_title/chapter_title/
    pub fn chapter_dir(&self) -> PathBuf {
        self.output_dir
            .join(sanitize_filename(&self.manga_title))
            .join(sanitize_filename(&self.chapter_title))
    }
}

/// Download queue managing multiple download tasks.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DownloadQueue {
    pub tasks: Vec<DownloadTask>,
}

impl DownloadQueue {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, task: DownloadTask) {
        self.tasks.push(task);
    }

    pub fn pending_count(&self) -> usize {
        self.tasks
            .iter()
            .filter(|t| t.status == DownloadStatus::Pending)
            .count()
    }

    pub fn completed_count(&self) -> usize {
        self.tasks
            .iter()
            .filter(|t| t.status == DownloadStatus::Completed)
            .count()
    }

    pub fn failed_count(&self) -> usize {
        self.tasks
            .iter()
            .filter(|t| t.status == DownloadStatus::Failed)
            .count()
    }

    pub fn next_pending(&mut self) -> Option<&mut DownloadTask> {
        self.tasks
            .iter_mut()
            .find(|t| t.status == DownloadStatus::Pending)
    }

    pub fn clear_completed(&mut self) {
        self.tasks.retain(|t| t.status != DownloadStatus::Completed);
    }

    pub fn retry_failed(&mut self) {
        for task in &mut self.tasks {
            if task.status == DownloadStatus::Failed {
                task.status = DownloadStatus::Pending;
                task.error = None;
            }
        }
    }

    pub fn len(&self) -> usize {
        self.tasks.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tasks.is_empty()
    }

    pub fn load(path: &Path) -> Self {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)
    }
}

fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            _ => c,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_progress() {
        let mut task = DownloadTask::new("Manga", "Ch 1", "ch1", PathBuf::from("/dl"), 10);
        assert_eq!(task.progress(), 0.0);
        task.mark_page_done();
        assert!((task.progress() - 0.1).abs() < 0.01);
    }

    #[test]
    fn test_task_completion() {
        let mut task = DownloadTask::new("Manga", "Ch 1", "ch1", PathBuf::from("/dl"), 2);
        task.mark_page_done();
        assert!(!task.is_complete());
        task.mark_page_done();
        assert!(task.is_complete());
        assert_eq!(task.status, DownloadStatus::Completed);
    }

    #[test]
    fn test_task_failure() {
        let mut task = DownloadTask::new("Manga", "Ch 1", "ch1", PathBuf::from("/dl"), 10);
        task.mark_failed("Network error".into());
        assert_eq!(task.status, DownloadStatus::Failed);
        assert_eq!(task.error, Some("Network error".into()));
    }

    #[test]
    fn test_chapter_dir() {
        let task = DownloadTask::new(
            "My Manga",
            "Chapter 1",
            "ch1",
            PathBuf::from("/downloads"),
            5,
        );
        let dir = task.chapter_dir();
        assert_eq!(dir, PathBuf::from("/downloads/My Manga/Chapter 1"));
    }

    #[test]
    fn test_sanitize_filename() {
        assert_eq!(sanitize_filename("normal"), "normal");
        assert_eq!(sanitize_filename("a/b\\c:d"), "a_b_c_d");
        assert_eq!(sanitize_filename("file?.txt"), "file_.txt");
    }

    #[test]
    fn test_queue_operations() {
        let mut q = DownloadQueue::new();
        q.add(DownloadTask::new("M", "C1", "c1", PathBuf::from("/dl"), 5));
        q.add(DownloadTask::new("M", "C2", "c2", PathBuf::from("/dl"), 5));
        assert_eq!(q.len(), 2);
        assert_eq!(q.pending_count(), 2);
        assert_eq!(q.completed_count(), 0);
    }

    #[test]
    fn test_queue_next_pending() {
        let mut q = DownloadQueue::new();
        q.add(DownloadTask::new("M", "C1", "c1", PathBuf::from("/dl"), 2));
        q.add(DownloadTask::new("M", "C2", "c2", PathBuf::from("/dl"), 2));

        let task = q.next_pending().unwrap();
        assert_eq!(task.chapter_title, "C1");
        task.mark_page_done();
        task.mark_page_done(); // completes
        assert_eq!(q.completed_count(), 1);

        let task2 = q.next_pending().unwrap();
        assert_eq!(task2.chapter_title, "C2");
    }

    #[test]
    fn test_queue_clear_completed() {
        let mut q = DownloadQueue::new();
        q.add(DownloadTask::new("M", "C1", "c1", PathBuf::from("/dl"), 0));
        q.tasks[0].status = DownloadStatus::Completed;
        q.add(DownloadTask::new("M", "C2", "c2", PathBuf::from("/dl"), 5));
        q.clear_completed();
        assert_eq!(q.len(), 1);
        assert_eq!(q.tasks[0].chapter_title, "C2");
    }

    #[test]
    fn test_queue_retry_failed() {
        let mut q = DownloadQueue::new();
        q.add(DownloadTask::new("M", "C1", "c1", PathBuf::from("/dl"), 5));
        q.tasks[0].mark_failed("timeout".into());
        assert_eq!(q.failed_count(), 1);
        q.retry_failed();
        assert_eq!(q.failed_count(), 0);
        assert_eq!(q.pending_count(), 1);
    }

    #[test]
    fn test_queue_save_load() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("queue.json");
        let mut q = DownloadQueue::new();
        q.add(DownloadTask::new(
            "Test Manga",
            "Ch 1",
            "ch1",
            PathBuf::from("/dl"),
            10,
        ));
        q.save(&path).unwrap();
        let loaded = DownloadQueue::load(&path);
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded.tasks[0].manga_title, "Test Manga");
    }
}
