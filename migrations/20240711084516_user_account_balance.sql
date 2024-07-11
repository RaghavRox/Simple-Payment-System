-- Add migration script here
ALTER TABLE user_credentials ADD COLUMN balance BIGINT NOT NULL DEFAULT 0;
