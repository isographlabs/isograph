-- Test database schema for Phase 1 SQL support
-- Simple schema matching the Isograph test cases

CREATE TABLE IF NOT EXISTS users (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    email TEXT NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Sample data
INSERT INTO users (id, name, email) VALUES
    (1, 'Alice Johnson', 'alice@example.com'),
    (2, 'Bob Smith', 'bob@example.com'),
    (3, 'Carol Williams', 'carol@example.com'),
    (4, 'David Brown', 'david@example.com'),
    (5, 'Eve Davis', 'eve@example.com');

-- Additional table for future Phase 3 JOIN testing
CREATE TABLE IF NOT EXISTS posts (
    id INTEGER PRIMARY KEY,
    user_id INTEGER NOT NULL,
    title TEXT NOT NULL,
    content TEXT NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (user_id) REFERENCES users(id)
);

INSERT INTO posts (id, user_id, title, content) VALUES
    (1, 1, 'First Post', 'Hello from Alice!'),
    (2, 1, 'Second Post', 'Another post by Alice'),
    (3, 2, 'Bob''s Thoughts', 'Some thoughts from Bob'),
    (4, 3, 'Carol''s Update', 'Update from Carol'),
    (5, 2, 'More from Bob', 'Bob posts again');
