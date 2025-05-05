use indicatif::ProgressStyle;
use std::sync::LazyLock;
use std::{
    collections::HashMap,
    fs::File,
    io::{self},
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use crate::types::Version;

pub static PB_BAR_STYLE: LazyLock<ProgressStyle> = LazyLock::new(|| {
    ProgressStyle::with_template("[{elapsed_precise}] {prefix:20} {bar:60} {pos:>7}/{len:7} {msg}")
        .unwrap()
});

pub static PB_SPINNER_STYLE: LazyLock<ProgressStyle> = LazyLock::new(|| {
    ProgressStyle::with_template("[{elapsed_precise}] {prefix:20} {spinner} {msg}").unwrap()
});

pub struct InferencePerformanceTracker {
    save_dir: PathBuf,
    map: HashMap<&'static str, String>,
    curr_instant: Instant,
    started: Instant,
}

impl InferencePerformanceTracker {
    pub fn new(save_dir: &Path) -> Self {
        Self {
            save_dir: save_dir.to_path_buf(),
            map: HashMap::new(),
            curr_instant: Instant::now(),
            started: Instant::now(),
        }
    }

    fn record_instant(&mut self, key: &'static str) {
        self.map.insert(
            key,
            format!("{:.2}", self.curr_instant.elapsed().as_secs_f32()),
        );
    }

    fn reset_instant(&mut self) {
        self.curr_instant = Instant::now();
    }

    pub fn done_loading(&mut self, versions: &Vec<Version>) {
        self.record_instant("load_versions_rt");

        let n = versions.len();
        self.map.insert("no_versions", n.to_string());
        let total_files: usize = versions.iter().map(|v| v.files.len()).sum();
        self.map
            .insert("avg_files_per_version", (total_files / n).to_string());

        self.reset_instant();
    }

    pub fn done_inferring(&mut self) {
        self.record_instant("infer_rt");
        self.reset_instant();
    }

    pub fn done_saving(&mut self) {
        self.record_instant("saving_rt");
        self.reset_instant();
    }

    pub fn finished(&mut self, filename: String) -> io::Result<()> {
        self.curr_instant = self.started;
        self.record_instant("total_rt");

        // Save trace to JSON file
        serde_json::to_writer(File::create(self.save_dir.join(filename))?, &self.map).unwrap();

        Ok(())
    }

    pub fn elapsed(&self) -> Duration {
        self.started.elapsed()
    }
}
