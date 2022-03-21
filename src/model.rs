use std::{
    collections::HashMap,
    fs::{File, OpenOptions},
};

use indexmap::IndexMap;
use ron::ser::PrettyConfig;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct Job {
    pub points: IndexMap<u16, u16>,
}

impl Job {
    pub fn interpolate(&mut self, player_count: u16) -> f64 {
        self.points.sort_keys();
        if let Some(amount) = self.points.get(&player_count) {
            return *amount as f64
        }
        // No existing value, interpolate
        let mut plausible_bottom: Option<(u16, u16)> = None;
        let mut bottom: Option<(u16, u16)> = None;
        let mut top: Option<(u16, u16)> = None;
        for (point_player_count, point_amount) in self.points.iter() {
            if player_count < *point_player_count {
                plausible_bottom = Some((*point_player_count, *point_amount));
            } else {
                bottom = plausible_bottom;
                top = Some((*point_player_count, *point_amount));
                break
            }
        }

        match (bottom, top) {
            (Some((bottom_player_count, bottom_amount)), Some((top_player_count, top_amount))) => {
                let player_diff = top_player_count as f64 - bottom_player_count as f64;
                let amount_diff = top_amount as f64 - bottom_amount as f64;
                let scale = amount_diff / player_diff;
                bottom_amount as f64 + (player_count - bottom_player_count) as f64 * scale
            },
            (Some((bottom_player_count, bottom_amount)), None) => {
                let player_diff = bottom_player_count as f64;
                let amount_diff = bottom_amount as f64;
                let scale = amount_diff / player_diff;
                bottom_amount as f64 + (player_count - bottom_player_count) as f64 * scale
            },
            (None, Some((top_player_count, top_amount))) => {
                let player_diff = top_player_count as f64;
                let amount_diff = top_amount as f64;
                let scale = amount_diff / player_diff;
                player_count as f64 * scale
            },
            (None, None) => {
                0.
            },
        }
    }
}

/// Path to the job configuration file.
const JOBS_PATH: &'static str = r"jobs.ron";

pub type Jobs = HashMap<u64, Job>;

/// Loads jobs from file.
pub fn load_jobs() -> Jobs {
    if let Ok(jobs_file) = File::open(JOBS_PATH) {
        if let Ok(jobs) = ron::de::from_reader(jobs_file) {
            return jobs;
        }
    }
    HashMap::new()
}

/// Save jobs to file.
pub fn save_jobs(jobs: Jobs) {
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

const EXCLUDED_PATH: &'static str = r"excluded.ron";

/// Loads excluded role from file.
pub fn load_excluded() -> Option<u64> {
    if let Ok(excluded_file) = File::open(EXCLUDED_PATH) {
        if let Ok(excluded) = ron::de::from_reader(excluded_file) {
            return Some(excluded);
        }
    }
    None
}

/// Save excluded role to file.
pub fn save_excluded(excluded: u64) {
    if let Ok(excluded_file) = OpenOptions::new()
        .write(true)
        .truncate(true)
        .create(true)
        .open(EXCLUDED_PATH)
    {
        let config = PrettyConfig::default();
        ron::ser::to_writer_pretty(excluded_file, &excluded, config).ok();
    }
}