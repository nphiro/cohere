use envy;
use once_cell::sync::{Lazy, OnceCell};
use serde::de::DeserializeOwned;
use std::{process, sync::Mutex};

static ENV_STATE: Lazy<Mutex<Env>> = Lazy::new(|| Mutex::new(Env::Local));

#[derive(Clone, PartialEq)]
pub enum Env {
    Local,
    Dev,
    Sit,
    Alpha,
    Beta,
    Uat,
    Staging,
    Prod,
}

impl From<Env> for String {
    fn from(env: Env) -> Self {
        match env {
            Env::Local => "local".into(),
            Env::Dev => "dev".into(),
            Env::Sit => "sit".into(),
            Env::Alpha => "alpha".into(),
            Env::Beta => "beta".into(),
            Env::Uat => "uat".into(),
            Env::Staging => "staging".into(),
            Env::Prod => "prod".into(),
        }
    }
}

pub fn value() -> Env {
    let env_state = ENV_STATE.lock().unwrap();
    env_state.clone()
}

static ENV_LOADED: OnceCell<()> = OnceCell::new();

pub fn parse<T>(config: &mut T)
where
    T: DeserializeOwned,
{
    ENV_LOADED.get_or_init(|| {
        prepare_env();
    });

    match envy::from_env::<T>() {
        Ok(parsed) => {
            *config = parsed;
        }
        Err(err) => {
            println!("Error parsing environment variables: {}", err);
            process::exit(1);
        }
    }
}

fn prepare_env() {
    let mut env = std::env::var("ENV").unwrap_or_default();

    if env.is_empty() {
        read_env_file();
        env = std::env::var("ENV").unwrap_or_default();
    }

    let mut env_state = ENV_STATE.lock().unwrap();

    *env_state = match env.as_str() {
        "local" => Env::Local,
        "dev" => Env::Dev,
        "sit" => Env::Sit,
        "alpha" => Env::Alpha,
        "beta" => Env::Beta,
        "uat" => Env::Uat,
        "staging" => Env::Staging,
        "prod" => Env::Prod,
        _ => {
            println!("Invalid environment: {}", env);
            process::exit(1);
        }
    };
}

fn read_env_file() {
    let env_file = std::env::var("ENV_FILE").unwrap_or(".env".into());

    std::fs::read_to_string(&env_file)
        .map(|content| {
            for line in content.lines() {
                if line.trim().is_empty() || line.starts_with('#') {
                    continue;
                }
                let mut parts = line.splitn(2, '=');
                if let Some(key) = parts.next() {
                    if let Some(value) = parts.next() {
                        unsafe {
                            std::env::set_var(key.trim(), value.trim());
                        }
                    }
                }
            }
        })
        .unwrap_or_else(|_| {
            println!("No {:?} file found", env_file);
            process::exit(1);
        });
}
