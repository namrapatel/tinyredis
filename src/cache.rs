use std::cmp::Ordering;
use std::collections::HashMap;

use std::time::{SystemTime, UNIX_EPOCH};

pub fn now() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis()


#[derive(Debug)]

struct Entry {
    value: String,
    ttl: Option<u64>,
    insertion_time: u128,
    frequency: u64,
    aging: u64,
}

pub struct Cache {
    cache: HashMap<String, Entry>,
    maximum: usize,
}
impl Cache {
    pub fn new(maximum: usize) -> Self {
        Self {
            maximum: maximum,
            cache: HashMap::with_capacity(maximum),
        }
    }

    pub fn update_aging(&mut self, key: &str) {
        for mut c in &mut self.cache {
            if c.0 != key {
                c.1.aging += 1;
            }
        }
    }

    pub fn get(&mut self, key: &str) -> Option<String> {
        self.update_aging(key);

        if let Some(entry) = self.cache.get_mut(key) {
            if let Some(ttl) = entry.ttl {
                if now() - entry.insertion_time > ttl.into() {
                    self.cache.remove(key);
                    return None;
                }
            }
            entry.frequency += 1;
            entry.aging = 1;
            Some(entry.value.clone())
        } else {
            None
        }
    }
    pub fn get_key(&mut self) -> (Vec<String>, Vec<String>) {

        let mut keys: Vec<String> = vec![];
        let mut vals: Vec<String> = vec![];
        for (k, entry) in self.cache.iter_mut() {
                keys.push(k.to_string());
                vals.push(entry.value.to_string());
                
                
            }
            (keys, vals)
        }

    pub fn set(&mut self, key: String, value: String, ttl: Option<u64>) -> Option<String> {
        println!("\n{:?}\n", "Keys Before");
        for c in &self.cache {
            println!("{:?}", c);
        }
        self.update_aging(&key);
        let result: Result<(String, String), ()> = match self.cache.keys().len().cmp(&self.maximum)
        {
            Ordering::Greater => Err(()),
            Ordering::Equal => {
                // LRU
                let mut maxAging = 0;
                let mut maxAgingKey = "";
                for c in &self.cache {
                    if c.1.aging > maxAging {
                        maxAging = c.1.aging;
                        maxAgingKey = c.0;
                    }
                }
                // LFU
                let mut minFrequency = std::u64::MAX;
                let mut minFrequencyKey = "";
                for c in &self.cache {
                    if c.1.frequency < minFrequency {
                        minFrequency = c.1.frequency;
                        minFrequencyKey = c.0;
                    }
                }
                // LRU policy
                Ok(("Equal".to_string(), maxAgingKey.to_string()))
            }
            // smaller
            _ => Ok(("Smaller".to_string(), "".to_string())),
        };
        match result {
            Ok(r) => {
                if r.0 == "Equal" {
                    self.remove(&r.1);
                }

                self.cache.insert(
                    key,
                    Entry {
                        value,
                        ttl,
                        insertion_time: now(),
                        frequency: 0,
                        aging: 1,
                    },
                );
                println!("\n{:?}\n", "Keys After");
                for c in &self.cache {
                    println!("{:?}", c);
                }
                Some("OK".to_string())
            }
            Err(_) => None,
        }
    }

    pub fn remove(&mut self, key: &str) -> Option<String> {
        let keys = key.trim().split(" ");
        let mut counter = 0;
        for k in keys {
            if (self.cache.contains_key(k)) {
                counter += 1;
                self.cache.remove(k);
            }
        }
        Some(counter.to_string())
    }
}
