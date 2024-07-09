use std::{env, sync::OnceLock};

#[allow(non_snake_case)]
pub struct Config {
    pub(crate) DATABASE_URL: String,
}
pub fn config() -> &'static Config {
    static INSTANCE: OnceLock<Config> = OnceLock::new();

    INSTANCE.get_or_init(|| Config {
        DATABASE_URL: read_env_var("DATABASE_URL"),
    })
}

fn read_env_var(var: &str) -> String {
    env::var(var).unwrap_or_else(|_| panic!("Unable to read the env variable {var}"))
}
