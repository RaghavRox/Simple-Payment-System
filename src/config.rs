use std::env;

#[allow(non_snake_case)]
pub struct Config {
    pub(crate) DATABASE_URL: String,
}

impl Config {
    pub fn init_from_env() -> Self {
        Config {
            DATABASE_URL: read_env_var("DATABASE_URL"),
        }
    }
}

fn read_env_var(var: &str) -> String {
    env::var(var).unwrap_or_else(|_| panic!("Unable to read the env variable {var}"))
}
