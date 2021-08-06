use std::fs::File;

use serde::Deserialize;

/// Qualification prefix
pub const QUALIFICATION_PREFIX: &str = "Qualified ";

#[derive(Deserialize, Debug)]
pub struct Job {
    pub name: String,
    pub minimum: u32,
    pub maximum: u32,
    pub weight: u64,
}

/// Path to the job configuration file.
const JOBS_PATH: &str = r"jobs.ron";

/// Loads jobs from file.
pub fn load_jobs() -> Vec<Job> {
    if let Ok(jobs_file) = File::open(JOBS_PATH) {
        if let Ok(jobs) = ron::de::from_reader(jobs_file) {
            return jobs;
        }
    }
    Vec::new()
}
