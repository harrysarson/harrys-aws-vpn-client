#![warn(clippy::pedantic)]
#![warn(unreachable_pub)]
#![deny(unsafe_code)]

mod cmd;
mod config;
mod saml_server;

use cmd::exec_ovpn_in_place;
use config::Config;
use std::{env, ffi::OsString, thread};

const SAML_SERVER_PORT: u16 = 35001;

fn main() -> ! {
    tracing_subscriber::fmt::init();

    let args: Vec<OsString> = env::args_os().collect();

    if args.len() != 2
        || args.iter().any(|arg| {
            arg.as_encoded_bytes()
                .get(0..1)
                .unwrap_or(b"")
                .starts_with(b"-")
        })
    {
        eprintln!("Usage: {} <FILE>", args[0].to_string_lossy());
        eprintln!();
        eprintln!("Arguments:");
        eprintln!("  <FILE>  Path to openvpn config downloaded from AWS.");
        std::process::exit(1);
    }

    let config = Config::new(&args[1]);

    let (aws_data, data) = thread::scope(|s| {
        let pwd_t = s.spawn(|| {
            tracing::info!("Connecting...");

            let auth = cmd::run_ovpn(&config, SAML_SERVER_PORT);

            match open::that(&auth.url) {
                Ok(()) => {}
                Err(e) => {
                    eprintln!("Opening URL error: {e:?}");
                    eprintln!("Failed to open <{}>. Try opening manually.", &auth.url);
                }
            }

            auth
        });
        let data_t = s.spawn(|| saml_server::run_server_for_saml(SAML_SERVER_PORT));

        (pwd_t.join().unwrap(), data_t.join().unwrap())
    });

    exec_ovpn_in_place(&config, &aws_data.ip, aws_data.port, &aws_data.pwd, &data);
}
