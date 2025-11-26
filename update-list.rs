#!/usr/bin/env rust-script
//! Adds new gem releases to the database.
//!
//! ```cargo
//! [dependencies]
//! postgres = { version = "0.19.12", features = ["with-time-0_3"] }
//! time = "0.3"
//! reqwest = { version = "0.12.24", features = ["blocking"] }
//! serde_json = "*"
//! ```

use postgres::{Client, NoTls};
use reqwest;
use serde_json::Value;
use time::PrimitiveDateTime;

const LATEST_PUSH_DATE: &str = "SELECT max(versions.created_at) FROM versions LIMIT 1";

const ADD_ROW: &str = "
INSERT INTO push_reviews (gem_name, gem_version, previous_version, version_created_at)
VALUES ($1, $2, $3, $4)
ON CONFLICT (gem_name, gem_version, version_created_at) DO NOTHING
";

fn get_previous_version(gem_name: &str, gem_version: &str) -> Option<String> {
    let url = format!("https://rubygems.org/api/v1/versions/{}.json", gem_name);
    let body = reqwest::blocking::get(&url).ok()?
        .text().ok()?;

    let val: Value = serde_json::from_str(&body).ok()?;
    let mut vals: &Vec<_> = val.as_array()?;
    let mut vals: Vec<_> = vals.into_iter().filter_map(|v| v.get("number")).map(|v| v.as_str().unwrap()).collect();
    vals.sort();

    let idx = vals.iter().position(|v| *v == gem_version)?;

    if idx == 0 {
        return None;
    }

    Some(vals[idx - 1].to_owned())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let pg_env = std::fs::read_to_string("pg-env")?;
    let mut parts = pg_env.split("=");
    parts.next().ok_or("no equal sign (=) found in pg-env file")?;
    let password = parts.next().ok_or("no value after equal sign (=) in pg-env file")?.trim();
    let mut client = Client::connect(&format!("host=localhost user=postgres password={} dbname=rubygems", password), NoTls)?;

    println!("Creating push_reviews table...");
    client.batch_execute("
        CREATE TABLE IF NOT EXISTS push_reviews (
            gem_name            text NOT NULL CHECK ( length(gem_name) < 100 ),
            gem_version         text NOT NULL CHECK ( length(gem_version) < 100 ),
            version_created_at  TIMESTAMP NOT NULL,
            previous_version    text CHECK ( length(previous_version) < 100 ),
            reviewed            boolean DEFAULT false,
            reviewer_gh_ids     integer[] DEFAULT array[]::integer[],
            UNIQUE (gem_name, gem_version, version_created_at)
        )
    ")?;

    println!("Determining end date...");
    let end_date: Option<PrimitiveDateTime> = client.query_one(LATEST_PUSH_DATE, &[])?.get(0);
    let end_date = end_date.unwrap();

    let query = format!(
        "SELECT
          versions.created_at,
          rubygems.name,
          versions.number
        FROM versions LEFT OUTER JOIN rubygems ON versions.rubygem_id = rubygems.id
        WHERE versions.created_at BETWEEN date_subtract('{end_date}', '1 week'::interval, 'UTC') AND '{end_date}'
        ORDER BY versions.created_at DESC
        LIMIT 10000",
        end_date=end_date,
    );

    println!("Fetching weekly pushes...");
    for row in client.query(&query, &[])? {
        let version_created_at: Option<PrimitiveDateTime> = row.get(0);
        let version_created_at = version_created_at.unwrap();
        let gem_name: &str = row.get(1);
        let gem_version: &str = row.get(2);

        println!("Determining previous version of {gem_name} {gem_version}");
        let previous_version = get_previous_version(gem_name, gem_version);

        println!("Adding: {gem_name} {gem_version} {version_created_at}");
        client.execute(ADD_ROW, &[&gem_name, &gem_version, &previous_version, &version_created_at])?;
        println!();

        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    Ok(())
}
