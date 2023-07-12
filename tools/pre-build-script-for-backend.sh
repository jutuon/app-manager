#!/bin/bash -eux

if ! command -v sqlx &> /dev/null; then
    echo "sqxl is not installed. Installing..."
    cargo install sqlx-cli@0.6.3 --no-default-features --features sqlite,rustls
fi

DATABASE_FILE=database/current/current.db

mkdir -p database/current

if [ -f "$DATABASE_FILE" ]; then
    echo "Deleting previous database..."
    rm "$DATABASE_FILE"
fi

sqlx database setup

echo "Script completed successfully!"
