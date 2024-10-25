use std::collections::HashSet;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;
use std::sync::LazyLock;

static BAD_PREFIXES: LazyLock<HashSet<String>> = LazyLock::new(|| {
    let mut set = HashSet::new();
    set.insert("auth-user-pass".to_string());
    set.insert("auth-federate".to_string());
    set.insert("auth-retry interact".to_string());
    set
});


pub(crate) struct Config {
    contents: String,
}


#[derive(Default)]
#[non_exhaustive]
pub(crate) struct SaveOpts {
    pub(crate) hide_remote: bool,
}


impl Config {
    pub(crate) fn new(p: impl AsRef<Path>) -> Config {
        let p = p.as_ref();
        let contents = std::fs::read_to_string(p).unwrap();
        Config {
            contents,
        }
    }

    pub(crate) fn save_config<P: AsRef<Path>>(&self, path: P, opts: &SaveOpts) {
        let path = path.as_ref();

        let mut file = BufWriter::new(
            File::create(path)
                .unwrap_or_else(|e| panic!("Failed to create {path:?}, error was {e:?}.")),
        );

        self.contents.lines().filter(|l| !has_key(l, opts)).for_each(|l| {
            writeln!(file, "{l}")
                .unwrap_or_else(|e| panic!("Failed writing to {path:?}, error was {e:?}."));
        });

        drop(file);

        tracing::info!("Saved at {:?}", &path);
    }

}

fn has_key(key: &str, opts: &SaveOpts   ) -> bool {
    for k in BAD_PREFIXES.iter() {
        if key.starts_with(k) {
            return true;
        }
    }

    if opts.hide_remote && key.starts_with("remote ") {
        return true;
    }

    false
}
