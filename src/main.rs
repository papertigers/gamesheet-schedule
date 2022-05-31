mod models;

use anyhow::Result;
use chrono::prelude::*;
use models::*;
use reqwest::{blocking as http, Url};
use serde::Serialize;
use std::{
    collections::HashMap,
    fs::OpenOptions,
    path::{Path, PathBuf},
};
use structopt::StructOpt;

static BASE_URL: &str = "https://gamesheet.app";

#[derive(Debug, StructOpt)]
#[structopt(name = "gamesheet-schedule", about = "Get a leagues schdeule")]
struct Opt {
    #[structopt(short = "i", long = "id", required = true)]
    id: u32,
    #[structopt(short = "t", long = "team")]
    team: Option<String>,
    #[structopt(parse(from_os_str), short = "o", long = "output", required = true)]
    output: PathBuf,
}

#[derive(Serialize, Debug)]
struct Game {
    home: String,
    visitor: String,
    scheduled_at: String,
    location: String,
}

#[derive(Serialize, Debug)]
struct Schedule {
    games: Vec<Game>,
    last_updated: String,
}

fn update_json_file<P: AsRef<Path>>(output_dir: P, schedule: &Schedule) -> Result<()> {
    let dir = output_dir.as_ref();
    let from_path = dir.join("schedule-temp.json");
    let to_path = dir.join("schedule.json");

    let mut temp_file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&from_path)?;

    serde_json::to_writer(&mut temp_file, schedule)?;
    std::fs::rename(from_path, to_path).map_err(|e| e.into())
}

fn main() -> Result<()> {
    let opts = Opt::from_args();

    let id = opts.id;
    let now: DateTime<Local> = Local::now();
    let since = now.format("%Y-%m-%dT00:00:00Z").to_string();
    let mut url = Url::parse(BASE_URL).unwrap();
    url.set_path(&format!("api/stats/v1/seasons/{id}/schedule"));
    url.set_query(Some(&format!("offset=0&limit=50&start_time_from={since}")));

    let json: GameSheet = http::get(url)?.json()?;

    let teams: HashMap<_, _> = json
        .included
        .iter()
        .filter_map(|i| match i {
            Included::Teams { id, attributes } => Some((id.to_string(), attributes.title.clone())),
            _ => None,
        })
        .collect();

    let games: Vec<Game> = json
        .included
        .iter()
        .filter_map(|i| match i {
            Included::ScheduledGames {
                attributes,
                relationships,
            } => {
                let home = match teams.get(&relationships.home_team.data.id) {
                    Some(t) => t,
                    None => return None,
                }
                .to_string();
                let visitor = match teams.get(&relationships.visitor_team.data.id) {
                    Some(t) => t,
                    None => return None,
                }
                .to_string();

                if let Some(ref team) = opts.team {
                    if home != *team && visitor != *team {
                        return None;
                    }
                }

                let location = attributes.location.to_string();
                // NOTE: this isn't actually UTC.
                // The API seems to return the local game time
                // "2022-05-27T20:30:00Z"  => America/New_York 830pm.
                let scheduled_at =
                    if let Ok(time) = attributes.scheduled_start_time.parse::<DateTime<Utc>>() {
                        time
                    } else {
                        return None;
                    };
                let scheduled_at = scheduled_at.format("%A %B %d %r").to_string();

                let game = Game {
                    home,
                    visitor,
                    scheduled_at,
                    location,
                };

                Some(game)
            }
            _ => None,
        })
        .collect();

    let last_updated = now.format("%A %B %d %r").to_string();
    let schedule = Schedule {
        games,
        last_updated,
    };

    update_json_file(&opts.output, &schedule)?;

    Ok(())
}
