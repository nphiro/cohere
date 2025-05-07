use envy;
use once_cell::sync::{Lazy, OnceCell};
use serde::de::DeserializeOwned;
use std::{process, sync::Mutex};

static ENV_LOADED: OnceCell<()> = OnceCell::new();

static ENV_STATE: Lazy<Mutex<String>> = Lazy::new(|| {
    Mutex::new(String::new())
});

pub fn parse<T>(config: &mut T)
where
    T: DeserializeOwned,
{
    ENV_LOADED.get_or_init(|| {
        prepare_env();
    });

    match envy::from_env::<T>() {
        Ok(parsed) => {
            // Replace the config with parsed values
            *config = parsed;
        }
        Err(err) => {
            println!("Error parsing environment variables: {}", err);
            process::exit(1);
        }
    }
}

fn prepare_env() {
    let env = std::env::var("ENV").unwrap_or_default();
    
    if env != "" {
        return;
    }

    let env_file = std::env::var("ENV_FILE").unwrap_or(".env".to_string());

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

    let mut env_state = ENV_STATE.lock().unwrap();
    let env = std::env::var("ENV").unwrap();

    if !["local", "dev", "staging", "production"].contains(&env.as_str()) {
        println!("Invalid environment value: {:?}", env);
        process::exit(1);
    }


    *env_state = env;

}

pub fn value() -> String {
    let env_state = ENV_STATE.lock().unwrap();
    env_state.clone()
}
