use crate::local_config::LocalConfig;
use crate::saml_server::Saml;
use lazy_static::lazy_static;
use std::env;
use std::ffi::OsString;
use std::fs::{remove_file, File};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use temp_dir::TempDir;
use tokio::io::AsyncBufReadExt;

const DEFAULT_PWD_FILE: &str = "./pwd.txt";

lazy_static! {
    static ref SHARED_DIR: String = std::env::var("SHARED_DIR").unwrap_or("./share".to_string());
    static ref OPENVPN_FILE: String =
        std::env::var("OPENVPN_FILE").unwrap_or("./openvpn/bin/openvpn".to_string());
}

pub struct ProcessInfo {
    pub pid: Mutex<Option<u32>>,
}

impl ProcessInfo {
    pub fn new() -> Self {
        Self {
            pid: Mutex::new(None),
        }
    }
}

#[derive(Debug)]
pub struct AwsSaml {
    pub url: String,
    pub pwd: String,
}

pub fn run_ovpn(config: impl AsRef<Path>, addr: String, port: u16) -> AwsSaml {
    let config = config.as_ref();
    let path = Path::new(SHARED_DIR.as_str()).join(DEFAULT_PWD_FILE);
    if !path.exists() {
        println!(
            "{:?} does not exist in {:?}!",
            path,
            env::current_dir().unwrap()
        );
    }
    let out = Command::new(OPENVPN_FILE.as_str())
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
        .spawn()
        .unwrap();

    let pid = out.id();
    let stdout = out.stdout.unwrap();

    let buf = BufReader::new(stdout);

    let mut addr = None::<String>;
    let mut pwd = None::<String>;

    for line in buf.lines() {
        let line = line.unwrap();
        tracing::info!("[{pid}] {line}");
        let auth_prefix = "AUTH_FAILED,CRV1";
        let prefix = "https://";

        if line.contains(auth_prefix) {
            tracing::info!("[{pid}] Found {line} redirect url");
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

pub async fn connect_ovpn(
    config: impl AsRef<Path>,
    addr: String,
    port: u16,
    saml: Saml,
    process_info: Arc<ProcessInfo>,
) -> i32 {
    let config = config.as_ref();
    let temp = TempDir::new().unwrap();
    let temp_pwd = temp.child("pwd.txt");

    if temp_pwd.exists() {
        remove_file(&temp_pwd).unwrap();
    }

    let mut save = File::create(&temp_pwd).unwrap();
    write!(save, "N/A\nCRV1::{}::{}\n", saml.pwd, saml.data).unwrap();

    let b = std::fs::canonicalize(temp_pwd).unwrap().clone();

    let mut out = tokio::process::Command::new("pkexec")
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
        .stdout(Stdio::piped())
        .current_dir(SHARED_DIR.as_str())
        .kill_on_drop(true)
        .spawn()
        .unwrap();

    let pid = out.id().unwrap();
    // Set pid
    {
        let mut stored_pid = process_info.pid.lock().unwrap();
        *stored_pid = Some(pid);
        LocalConfig::save_last_pid(Some(pid));
    }

    let stdout = out.stdout.take().unwrap();

    let buf = tokio::io::BufReader::new(stdout);
    let mut lines = buf.lines();

    let mut next = lines.next_line().await;

    loop {
        if let Ok(Some(line)) = next {
            tracing::info!("[{pid}] {line}");
        } else {
            break;
        }

        next = lines.next_line().await;
    }

    out.wait().await.unwrap().code().unwrap()
}

pub fn kill_openvpn(pid: u32) {
    if pid == 0 || pid == 1 {
        LocalConfig::save_last_pid(None);
        return;
    }

    let info = Command::new("ps")
        .arg("-o")
        .arg("cmd")
        .arg("-p")
        .arg(format!("{pid}"))
        .output()
        .unwrap();

    if let Ok(msg) = String::from_utf8(info.stdout) {
        let last = msg.lines().next_back();
        if let Some(last) = last {
            if !last.is_empty() && last.chars().next().is_some_and(|v| v == '/') {
                if last.contains("openvpn --config /")
                    && last.contains("--auth-user-pass /")
                    && last.ends_with("pwd.txt")
                {
                    let mut p = Command::new("pkexec")
                        .arg("kill")
                        .arg(format!("{pid}"))
                        .spawn()
                        .unwrap();

                    p.wait().unwrap();
                    LocalConfig::save_last_pid(None);
                } else {
                    LocalConfig::save_last_pid(None);
                }
            } else {
                LocalConfig::save_last_pid(None);
            }
        }
    }
}

fn rm_file_command(dir: &PathBuf) -> OsString {
    let mut str = OsString::new();
    str.push("/usr/bin/env rm ");
    str.push(dir);
    str
}
