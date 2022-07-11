mod models;

use anyhow::Result;
use chrono::{prelude::*, Duration};
use ics::{
    components::Property,
    escape_text,
    properties::{Categories, Description, DtEnd, DtStart, Location, Status, Summary},
    Event, ICalendar,
};
use models::*;
use reqwest::{blocking as http, Url};
use serde::Serialize;
use std::{
    collections::HashMap,
    fs::{File, OpenOptions},
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
    id: String,
    home: String,
    visitor: String,
    scheduled_at: DateTime<Utc>,
    scheduled_at_pretty: String,
    location: String,
}

#[derive(Serialize, Debug)]
struct Schedule {
    games: Vec<Game>,
    last_updated: String,
}

fn update_file<P, N, F>(output_dir: P, file_name: N, func: F) -> Result<()>
where
    P: AsRef<Path>,
    N: AsRef<str>,
    F: Fn(&mut File) -> Result<()>,
{
    let dir = output_dir.as_ref();
    let file = file_name.as_ref();
    let mut from_path = dir.join(file);
    from_path.set_extension("temp");
    let to_path = dir.join(file);

    let mut temp_file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&from_path)?;

    func(&mut temp_file)?;
    std::fs::rename(from_path, to_path).map_err(|e| e.into())
}

fn update_json_file<P: AsRef<Path>>(output_dir: P, schedule: &Schedule) -> Result<()> {
    update_file(output_dir, "schedule.json", |file| {
        serde_json::to_writer(file, schedule).map_err(|e| e.into())
    })
}

fn update_ics_file<P: AsRef<Path>>(output_dir: P, calendar: &ICalendar) -> Result<()> {
    update_file(output_dir, "schedule.ics", |file| {
        calendar.write(file).map_err(|e| e.into())
    })
}

fn create_ics(games: &[Game]) -> Result<ICalendar> {
    let mut calendar = ICalendar::new(
        "2.0",
        "-//Hoiday Lesiure//Rec League Calendar Version 1.0//EN",
    );
    calendar.push(Property::new("X-WR-CALNAME", "Drunkin' Uncles"));
    for game in games {
        let summary = format!("{} at {}", &game.visitor, &game.home);
        let desc = format!("{summary}\n{}", &game.location);
        let mut event = Event::new(&game.id, &game.scheduled_at_pretty);
        event.push(DtStart::new(
            game.scheduled_at.format("%Y%m%dT%H%M%SZ").to_string(),
        ));
        event.push(DtEnd::new(
            (game.scheduled_at + Duration::hours(1) + Duration::minutes(30))
                .format("%Y%m%dT%H%M%SZ")
                .to_string(),
        ));
        event.push(Status::confirmed());
        event.push(Summary::new(escape_text(summary)));
        event.push(Description::new(escape_text(desc)));
        event.push(Location::new(&game.location));
        event.push(Categories::new("HOCKEYGAME"));
        calendar.add_event(event);
    }

    Ok(calendar)
}

fn main() -> Result<()> {
    let opts = Opt::from_args();

    let id = opts.id;
    let now: DateTime<Local> = Local::now();
    let since = now.format("%Y-%m-%dT00:00:00Z").to_string();
    let mut url = Url::parse(BASE_URL).unwrap();
    url.set_path(&format!("api/stats/v1/seasons/{id}/schedule"));
    url.set_query(Some(&format!(
        "offset=0&limit=50&filter[start_time_from]={since}"
    )));

    let json: GameSheet = http::get(url)?.json()?;

    let teams: HashMap<_, _> = json
        .included
        .iter()
        .filter_map(|i| match i {
            Included::Teams { id, attributes } => Some((id.to_string(), attributes.title.clone())),
            _ => None,
        })
        .collect();

    let mut games: Vec<Game> = json
        .included
        .iter()
        .filter_map(|i| match i {
            Included::ScheduledGames {
                id,
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

                let id = id.to_string();
                let location = attributes.location.to_string();
                // NOTE: this isn't actually UTC.
                // The API seems to return the local game time
                // "2022-05-27T20:30:00Z"  => America/New_York 830pm.
                let naive = NaiveDateTime::parse_from_str(
                    &attributes.scheduled_start_time,
                    "%Y-%m-%dT%H:%M:%SZ",
                )
                .ok()?;
                let eastern = chrono_tz::America::New_York
                    .from_local_datetime(&naive)
                    .single()?;
                let scheduled_at = eastern.with_timezone(&Utc);
                let scheduled_at_pretty = eastern.format("%A %B %d %I:%M %p").to_string();

                let game = Game {
                    id,
                    home,
                    visitor,
                    scheduled_at,
                    scheduled_at_pretty,
                    location,
                };

                Some(game)
            }
            _ => None,
        })
        .collect();

    games.sort_by_key(|g| g.scheduled_at);
    let calendar = create_ics(&games)?;
    update_ics_file(&opts.output, &calendar)?;

    let last_updated = now.format("%A %B %d %r").to_string();
    let schedule = Schedule {
        games,
        last_updated,
    };
    update_json_file(&opts.output, &schedule)?;

    Ok(())
}
