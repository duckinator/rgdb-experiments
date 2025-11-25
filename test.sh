#!/usr/bin/env bash

DATE=2025.11.24

BASE_URL="https://s3-us-west-2.amazonaws.com/rubygems-dumps/"
PREFIX="production/public_postgresql"

SUFFIX=$(curl "$BASE_URL?prefix=$PREFIX" | xq -x //ListBucketResult/Contents/Key | sort | grep "$DATE")
FILE=$(basename $SUFFIX)

URL="https://s3-us-west-2.amazonaws.com/rubygems-dumps/production/public_postgresql/2025.11.24.2"

if [ ! -f "$FILE" ]; then
    echo "Downloading $FILE..."
    curl --progress-bar "${BASE_URL}${SUFFIX}" -o "$FILE"
fi

run() {
    echo
    echo "> $@"
    "$@"
}

export PGHOST=localhost
export PGPORT=5432
export PGUSER=postgres
export PGDATABASE=rubygems
source pg-env
export PGPASSWORD

run podman-compose kill --all
run podman-compose down
run podman-compose --env-file pg-env up --detach

sleep 5

run createdb
run psql -q -c "CREATE EXTENSION IF NOT EXISTS hstore;"

echo "Loading the data from $public_tar"
tar xOf "$FILE" public_postgresql/databases/PostgreSQL.sql.gz | \
  gunzip -c | \
  psql

echo
echo
echo
echo "http://localhost:8080/?pgsql=db&username=postgres&db=rubygems"
echo "Password: $PGPASSWORD"
