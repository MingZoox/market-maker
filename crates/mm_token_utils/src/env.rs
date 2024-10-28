pub fn get_env(key: &str, default_value: Option<String>) -> String {
    match default_value {
        Some(value) => std::env::var(key).unwrap_or(value),
        None => std::env::var(key).unwrap_or_else(|_| panic!("expect env {}", key)),
    }
}
