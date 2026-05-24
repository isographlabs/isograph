# Test Database Fixtures

SQLite test databases for Phase 1-3 SQL support testing.

## Setup

Initialize the test database:

```bash
./test-fixtures/databases/init-db.sh
```

This creates `test.db` with sample data.

## Schema

### users table

| Column     | Type                | Description        |
| ---------- | ------------------- | ------------------ |
| id         | INTEGER PRIMARY KEY | User ID            |
| name       | TEXT                | User name          |
| email      | TEXT                | User email         |
| created_at | TIMESTAMP           | Creation timestamp |

**Sample data**: 5 users (Alice, Bob, Carol, David, Eve)

### posts table

| Column     | Type                | Description             |
| ---------- | ------------------- | ----------------------- |
| id         | INTEGER PRIMARY KEY | Post ID                 |
| user_id    | INTEGER             | Foreign key to users.id |
| title      | TEXT                | Post title              |
| content    | TEXT                | Post content            |
| created_at | TIMESTAMP           | Creation timestamp      |

**Sample data**: 5 posts from various users

## Usage

### Inspect the database

```bash
sqlite3 test-fixtures/databases/test.db
```

### Run queries

```bash
# Simple SELECT
sqlite3 test-fixtures/databases/test.db 'SELECT * FROM users;'

# Filtered query (Phase 2)
sqlite3 test-fixtures/databases/test.db 'SELECT * FROM users WHERE id = 1;'

# JOIN query (Phase 3)
sqlite3 test-fixtures/databases/test.db '
  SELECT u.name, p.title
  FROM users u
  JOIN posts p ON u.id = p.user_id
  WHERE u.id = 1;
'
```

## Testing with isograph-server

The server will use datafusion-federation to connect to this SQLite database for executing queries.

Phase 1 queries (simple SELECT):

```sql
SELECT id, name FROM users;
```

Phase 2 queries (with WHERE):

```sql
SELECT id, name FROM users WHERE id = $1;
```

Phase 3 queries (with JOIN):

```sql
SELECT u.id, u.name, p.title
FROM users u
LEFT JOIN posts p ON u.id = p.user_id;
```
