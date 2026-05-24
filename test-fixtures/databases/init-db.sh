#!/usr/bin/env bash
# Initialize SQLite test database with schema and sample data

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DB_PATH="${SCRIPT_DIR}/test.db"

# Remove existing database if it exists
if [ -f "$DB_PATH" ]; then
    echo "Removing existing database..."
    rm "$DB_PATH"
fi

# Create new database and apply schema
echo "Creating database at: $DB_PATH"
sqlite3 "$DB_PATH" < "${SCRIPT_DIR}/schema.sql"

echo "✅ Database initialized successfully!"
echo ""
echo "To inspect the database:"
echo "  sqlite3 $DB_PATH"
echo ""
echo "To run queries:"
echo "  sqlite3 $DB_PATH 'SELECT * FROM users;'"
