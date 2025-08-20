use chrono::{Datelike, Local};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct WordleManifest {
    pub id: i32,
    pub solution: String,
    pub days_since_launch: i32,
    pub editor: Option<String>,
}

fn daily_manifest_url() -> String {
    let local = Local::now();
    format!(
        "https://www.nytimes.com/svc/wordle/v2/{:04}-{:02}-{:02}.json",
        local.year(),
        local.month(),
        local.day()
    )
}

pub fn daily_manifest() -> anyhow::Result<WordleManifest> {
    Ok(reqwest::blocking::get(daily_manifest_url())?.json::<WordleManifest>()?)
}
