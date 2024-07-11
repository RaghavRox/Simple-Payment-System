-- Add migration script here
CREATE TABLE transactions(
    transaction_id uuid PRIMARY KEY DEFAULT gen_random_uuid(),
    from_user TEXT NOT NULL,
    to_user TEXT NOT NULL,
    created_at timestamptz default CURRENT_TIMESTAMP NOT NULL,

    FOREIGN KEY (from_user) REFERENCES user_credentials(username),
    FOREIGN KEY (to_user) REFERENCES user_credentials(username)
);
