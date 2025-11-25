#!/usr/bin/env rust-script
//! Adds new gem releases to the database.
//!
//! ```cargo
//! [dependencies]
//! postgres = "0.19.12"
//! ```

use postgres::{Client, NoTls};

const LATEST_PUSH_DATE: &str = "SELECT min(versions.created_at) FROM versions LIMIT 1";

const WEEKLY_PUSHES: &str = "
SELECT
  versions.created_at,
  rubygems.name,
  versions.canonical_number
FROM versions LEFT OUTER JOIN rubygems ON versions.rubygem_id = rubygems.id
WHERE versions.created_at BETWEEN date_subtract({end_date}, '1 week'::interval, 'UTC') AND {end_date}
ORDER BY versions.created_at DESC
LIMIT 10000
";

    const ADD_ROW: &str = "
INSERT INTO push_reviews (gem_host, gem_name, gem_version, version_created_at)
VALUES ($1, $2, $3, $4)
ON CONFLICT (gem_host, gem_name, gem_version, version_created_at) DO NOTHING
";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let pg_env = std::fs::read_to_string("pg-env")?;
    let mut parts = pg_env.split("=");
    parts.next().ok_or("no equal sign (=) found in pg-env file")?;
    let password = parts.next().ok_or("no value after equal sign (=) in pg-env file")?.trim();
    let mut client = Client::connect(&format!("host=localhost user=postgres password={}", password), NoTls)?;

    client.batch_execute("
      CREATE TABLE IF NOT EXISTS push_reviews (
          gem_host            text NOT NULL,
          gem_name            text NOT NULL CHECK ( length(gem_name) < 100 ),
          gem_version         text NOT NULL CHECK ( length(gem_version) < 100 ),
          version_created_at  TIMESTAMP NOT NULL,
          reviewed            boolean DEFAULT false,
          reviewed_by_gh_ids  integer[] DEFAULT []::integer[],
      )
    ")?;

    let end_date_rows = client.query(LATEST_PUSH_DATE, &[])?;
    let end_date: &str = end_date_rows.get(0).ok_or("Couldn't find end date")?.get(0);

    for row in client.query(WEEKLY_PUSHES, &[&end_date])? {
        println!("{:?}", row);
    }

    Ok(())
}
