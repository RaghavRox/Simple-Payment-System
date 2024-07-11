-- Add migration script here
ALTER TABLE transactions ADD COLUMN amount int NOT NULL;
