use std::{
    collections::HashMap,
    fs::{File, OpenOptions},
};

use ron::ser::PrettyConfig;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct Job {
    pub name: String,
    pub minimum: u32,
    pub maximum: u32,
    pub proportion: f64,
}

/// Path to the job configuration file.
const JOBS_PATH: &str = r"jobs.ron";

/// Loads jobs from file.
pub fn load_jobs() -> HashMap<String, Job> {
    if let Ok(jobs_file) = File::open(JOBS_PATH) {
        if let Ok(jobs) = ron::de::from_reader(jobs_file) {
            return jobs;
        }
    }
    HashMap::new()
}

/// Save jobs to file.
pub fn save_jobs(jobs: HashMap<String, Job>) {
    if let Ok(jobs_file) = OpenOptions::new()
        .write(true)
        .truncate(true)
        .create(true)
        .open(JOBS_PATH)
    {
        let config = PrettyConfig::default();
        ron::ser::to_writer_pretty(jobs_file, &jobs, config).ok();
    }
}
