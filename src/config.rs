use lazy_static::lazy_static;
use std::collections::HashSet;
use std::fs::File;
use std::io::Write;
use std::ops::Deref;
use std::path::Path;

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

pub(crate) struct Config {
    contents: String,
}

pub(crate) struct Pwd {
    pwd: String,
}
impl Pwd {
    pub(crate) fn new(pwd: String) -> Self {
        Self { pwd }
    }
}

impl Deref for Pwd {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.pwd
    }
}

impl Config {
    pub(crate) fn new(p: impl AsRef<Path>) -> Config {
        Config {
            contents: std::fs::read_to_string(p).unwrap(),
        }
    }

    pub(crate) fn save_config<P: AsRef<Path>>(&self, path: P) {
        let path = path.as_ref();

        let new_contents = self
            .contents
            .lines()
            .filter(|l| !has_key(l))
            .map(std::string::ToString::to_string)
            .collect::<Vec<String>>()
            .join("\n");

        let mut file = File::create(path).unwrap();
        write!(file, "{new_contents}").unwrap();
        tracing::info!("Saved at {:?}", &path);
    }

    pub(crate) fn get_remote(&self) -> (String, u16) {
        self
            .contents
            .lines()
            .filter(|p| p.starts_with("remote "))
            .map(|p| {
                let addr = p["remote ".len()..p.rfind(' ').unwrap()].to_string();
                let port = p[p.rfind(' ').unwrap() + 1..].parse::<u16>().unwrap();
                (addr, port)
            })
            .next()
            .unwrap()
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
