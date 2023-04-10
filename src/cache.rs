use std::collections::HashMap;

use std::time::{SystemTime, UNIX_EPOCH};

pub fn now() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis()
}

#[derive(Debug)]
struct Entry {
    value: String,
    ttl: Option<u64>,
    insertion_time: u128,
}

#[derive(Debug)]
pub struct Cache {
    cache: HashMap<String, Entry>,
}

impl Cache {
    pub fn new() -> Self {
        Self {
            cache: HashMap::with_capacity(128),
        }
    }

    pub fn get(&mut self, key: &str) -> Option<String> {
        match self.cache.get(key) {
            Some(entry) => {
                if let Some(ttl) = entry.ttl {
                    if now() - entry.insertion_time > ttl.into() {
                        self.cache.remove(key);

                        return None;
                    }
                }

                Some(entry.value.clone())
            }
            None => None,
        }
    }

    pub fn set(&mut self, key: String, value: String, ttl: Option<u64>) {
        self.cache.insert(
            key,
            Entry {
                value,
                ttl,
                insertion_time: now(),
            },
        );
    }
}