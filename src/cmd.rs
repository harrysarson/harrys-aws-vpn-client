use crate::config::Config;
use crate::config::SaveOpts;
use crate::saml_server::Saml;
use std::env;
use std::ffi::OsString;
use std::fs::File;
use std::io::BufRead;
use std::io::Write;
use std::net::IpAddr;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::LazyLock;
use std::thread;
use std::time::Duration;
use tempfile::TempDir;

static OPENVPN_FILE: LazyLock<String> =
    LazyLock::new(|| std::env::var("OPENVPN_FILE").unwrap_or("./openvpn/bin/openvpn".to_string()));

#[derive(Debug)]
pub(crate) struct AwsSaml {
    pub(crate) ip: String,
    pub(crate) port: u16,
    pub(crate) url: String,
    pub(crate) pwd: String,
}

struct StandardArgs {
    _temp: TempDir,
    password: PathBuf,
    config: PathBuf,
}

#[derive(Default)]
#[non_exhaustive]
pub(crate) struct ArgOpts {
    pub(crate) hide_remote: bool,
    pub(crate) keep_files: bool,
}

impl StandardArgs {
    fn new(password_contents: &str, config: &Config, opts: &ArgOpts) -> Self {
        let temp = tempfile::Builder::new()
            .keep(opts.keep_files)
            .tempdir()
            .unwrap();
        let password = temp.path().join("pwd.txt");
        let config_file = temp.path().join("config.ovpn");
        config.save_config(
            &config_file,
            &SaveOpts {
                hide_remote: opts.hide_remote,
            },
        );

        let mut save = File::create(&password).unwrap();
        writeln!(save, "{password_contents}").unwrap();

        Self {
            _temp: temp,
            password,
            config: config_file,
        }
    }

    fn args(&self) -> Vec<OsString> {
        vec![
            "--config".into(),
            self.config.clone().into(),
            "--auth-user-pass".into(),
            self.password.clone().into(),
            "--route-up".into(),
            rm_file_command(&std::fs::canonicalize(&self.password).unwrap().clone()),
            "--script-security".into(),
            "2".into(),
            "--verb".into(),
            "3".into(),
            "--proto".into(),
            "udp".into(),
        ]
    }
}

pub(crate) fn run_ovpn(config: &Config, saml_server_port: u16) -> AwsSaml {
    let standard_args = StandardArgs::new(
        &format!("N/A\nACS::{saml_server_port}\n"),
        config,
        &ArgOpts {
            ..Default::default()
        },
    );

    let mut command = std::process::Command::new(OPENVPN_FILE.as_str());
    let command = command.args(standard_args.args()).stdout(Stdio::piped());

    tracing::debug!("Running {:?}", command);

    let mut child = command.spawn().unwrap();

    let span = tracing::debug_span!("openvpn");

    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) => thread::sleep(Duration::from_millis(10)),
            Err(e) => panic!("Waiting on openvpn failed with {e:?}"),
        }
    }

    let out = child.wait_with_output().unwrap();

    let stdout = out.stdout;

    let mut addr = None::<String>;
    let mut pwd = None::<String>;
    let mut ip = None::<String>;
    let mut port = None::<u16>;

    for line in stdout.lines() {
        fn extract_port(s: &str) -> &str {
            s.char_indices()
                .find(|(_, c)| !c.is_numeric())
                .map_or(s, move |(i, _)| &s[..i])
        }

        let line = line.unwrap();
        tracing::info!(parent: &span, "{line}");
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

        if line.contains("[AF_INET]") {
            (ip, port) = {
                let find_start = line.find("[AF_INET]").unwrap() + "[AF_INET]".len();
                let line = &line[find_start..];
                let colon_pos = line.find(':').unwrap();
                let ip = line[..colon_pos].parse::<IpAddr>().unwrap().to_string();

                let port_s = extract_port(&line[(colon_pos + 1)..]);
                tracing::debug!("Found ip: {ip} and port {port_s} on line {line}");
                let port = port_s.parse::<u16>().unwrap();

                (Some(ip), Some(port))
            };
        }
    }

    let ret = AwsSaml {
        ip: ip.unwrap(),
        port: port.unwrap(),
        url: addr.unwrap(),
        pwd: pwd.unwrap(),
    };
    tracing::debug!("Found AWS Saml login info: {ret:?}");
    ret
}

pub(crate) fn exec_ovpn_in_place(
    config: &Config,
    addr: &str,
    port: u16,
    pwd: &str,
    saml: &Saml,
) -> ! {
    const KEEP_FILES: bool = false;
    let standard_args = StandardArgs::new(
        &format!("N/A\nCRV1::{}::{}\n", pwd, saml.data),
        config,
        &ArgOpts {
            hide_remote: true,
            keep_files: KEEP_FILES,
            ..Default::default()
        },
    );

    tracing::info!("Replacing process with openvpn");

    let cmd = env::var_os("PKEXEC").unwrap_or("pkexec".into());
    let mut cmd = cargo_util::ProcessBuilder::new(cmd);

    cmd.arg(OPENVPN_FILE.as_str())
        .args(&standard_args.args())
        .arg("--auth-nocache")
        .arg("--inactive")
        .arg("3600")
        .arg("--remote")
        .arg(addr)
        .arg(port.to_string());

    if KEEP_FILES {
        println!("{cmd}");
        panic!();
    }

    cmd.exec_replace().unwrap();

    unreachable!();
}

fn rm_file_command(dir: &PathBuf) -> OsString {
    let mut str = OsString::new();
    str.push("/usr/bin/env rm ");
    str.push(dir);
    str
}
