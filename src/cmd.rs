use crate::saml_server::Saml;
use std::env;
use std::ffi::OsString;
use std::fs::{remove_file, File};
use std::io::BufRead;
use std::path::{Path, PathBuf};
use std::process::{ Stdio};
use std::io::Write;
use std::time::Duration;
use lazy_static::lazy_static;
use temp_dir::TempDir;

const DEFAULT_PWD_FILE: &str = "./pwd.txt";

lazy_static! {
    static ref SHARED_DIR: String = std::env::var("SHARED_DIR").unwrap_or("./share".to_string());
    static ref OPENVPN_FILE: String =
        std::env::var("OPENVPN_FILE").unwrap_or("./openvpn/bin/openvpn".to_string());
}


#[derive(Debug)]
pub(crate) struct AwsSaml {
    pub(crate) url: String,
    pub(crate) pwd: String,
}

pub(crate) async fn run_ovpn(config: impl AsRef<Path>, addr: String, port: u16) -> AwsSaml {
    let config = config.as_ref();
    let path = Path::new(SHARED_DIR.as_str()).join(DEFAULT_PWD_FILE);
    if !path.exists() {
        println!(
            "{:?} does not exist in {:?}!",
            path,
            env::current_dir().unwrap()
        );
    }
    let fut = tokio::process::Command::new(OPENVPN_FILE.as_str())
        .arg("--config")
        .arg(config)
        .arg("--verb")
        .arg("3")
        .arg("--proto")
        .arg("udp")
        .arg("--remote")
        .arg(addr)
        .arg(format!("{port}"))
        .arg("--auth-user-pass")
        .arg(DEFAULT_PWD_FILE)
        .stdout(Stdio::piped())
        .current_dir(SHARED_DIR.as_str())
        .kill_on_drop(true)
        .output();

    let out = tokio::time::timeout(Duration::from_secs(30), fut).await.unwrap().unwrap();

    let stdout = out.stdout;


    let mut addr = None::<String>;
    let mut pwd = None::<String>;

    for line in stdout.lines() {
        let line = line.unwrap();
        tracing::info!("[openvpn] {line}");
        let auth_prefix = "AUTH_FAILED,CRV1";
        let prefix = "https://";

        if line.contains(auth_prefix) {
            tracing::info!("[openvpn] Found {line} redirect url");
            let find = line.find(prefix).unwrap();
            addr = Some(line[find..].to_string());

            let auth_find = line
                .find(auth_prefix)
                .map(|v| v + auth_prefix.len() + 1)
                .unwrap();

            let sub = &line[auth_find..find - 1];
            let e = sub.split(':').nth(1).unwrap();
            pwd = Some(e.to_string());
        }
    }

    AwsSaml {
        url: addr.unwrap(),
        pwd: pwd.unwrap(),
    }
}

pub(crate) fn exec_ovpn_in_place(
    config: impl AsRef<Path>,
    addr: String,
    port: u16,
    pwd: &str,
    saml: &Saml,
) -> ! {
    let config = config.as_ref();
    let temp = TempDir::new().unwrap();
    let temp_pwd = temp.child("pwd.txt");

    if temp_pwd.exists() {
        remove_file(&temp_pwd).unwrap();
    }

    let mut save = File::create(&temp_pwd).unwrap();
    write!(save, "N/A\nCRV1::{}::{}\n", pwd, saml.data).unwrap();
    drop(save);

    let b = std::fs::canonicalize(temp_pwd).unwrap().clone();

    cargo_util::ProcessBuilder::new("pkexec")
        .arg(OPENVPN_FILE.as_str())
        .arg("--config")
        .arg(config)
        .arg("--verb")
        .arg("3")
        .arg("--auth-nocache")
        .arg("--inactive")
        .arg("3600")
        .arg("--proto")
        .arg("udp")
        .arg("--remote")
        .arg(addr)
        .arg(format!("{port}"))
        .arg("--script-security")
        .arg("2")
        .arg("--route-up")
        .arg(rm_file_command(&b))
        .arg("--auth-user-pass")
        .arg(b)
        .cwd(SHARED_DIR.as_str())
        .exec_replace()
        .unwrap();

    unreachable!();
}

fn rm_file_command(dir: &PathBuf) -> OsString {
    let mut str = OsString::new();
    str.push("/usr/bin/env rm ");
    str.push(dir);
    str
}
