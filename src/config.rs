use lazy_static::lazy_static;
use std::collections::HashSet;
use std::fs::File;
use std::io::Write;
use std::net::IpAddr;
use std::path::Path;
use std::sync::{Arc, Mutex};

lazy_static! {
    static ref CLEAN_KEYS: HashSet<String> = {
        let mut set = HashSet::new();
        set.insert("remote ".to_string());
        set.insert("remote-random-hostname".to_string());
        set.insert("auth-user-pass".to_string());
        set.insert("auth-federate".to_string());
        set.insert("auth-retry interact".to_string());
        set
    };
}

pub struct Config {
    pub addresses: Arc<Mutex<Option<Vec<IpAddr>>>>,
    pub pwd: Arc<Mutex<Option<Pwd>>>,
    pub contents: Arc<Mutex<String>>,
}

pub struct Pwd {
    pub pwd: String,
}

impl Config {
    pub fn new() -> Config {
        Config {
            addresses: Arc::new(Mutex::new(None)),
            pwd: Arc::new(Mutex::new(None)),
            contents: Arc::new(Mutex::new(String::new())),
        }
    }

    pub fn save_config<P: AsRef<Path>>(&self, path: P) {
        let path = path.as_ref();

        let new_contents = self
            .contents
            .lock()
            .unwrap()
            .lines()
            .filter(|l| !has_key(l))
            .map(std::string::ToString::to_string)
            .collect::<Vec<String>>()
            .join("\n");

        let mut file = File::create(path).unwrap();
        write!(file, "{new_contents}").unwrap();
        tracing::info!("Saved at {:?}", &path);
    }

    pub fn get_remote(&self) -> (String, u16) {
        return self
            .contents
            .lock()
            .unwrap()
            .lines()
            .filter(|p| p.starts_with("remote "))
            .map(|p| {
                let addr = p["remote ".len()..p.rfind(' ').unwrap()].to_string();
                let port = p[p.rfind(' ').unwrap() + 1..].parse::<u16>().unwrap();
                (addr, port)
            })
            .next()
            .unwrap();
    }
}

fn has_key(key: &str) -> bool {
    for k in CLEAN_KEYS.iter() {
        if key.starts_with(k) {
            return true;
        }
    }

    false
}
