-- Add migration script here
CREATE TABLE user_credentials (
    username TEXT PRIMARY KEY,
    password TEXT NOT NULL,
    created_at timestamptz default CURRENT_TIMESTAMP NOT NULL
);
