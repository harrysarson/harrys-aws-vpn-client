#![warn(clippy::pedantic)]
#![warn(unreachable_pub)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::similar_names)]
#![allow(clippy::struct_field_names)]

mod cmd;
mod config;
mod dns;
mod manager;
mod saml_server;

use clap::Parser;
use cmd::exec_ovpn_in_place;
use config::Config;
use std::path::PathBuf;

#[derive(clap::Parser)]
struct Cli {
    file: PathBuf,
}

fn main() -> ! {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();

    let config = Config::new(&cli.file);

    let (addresses, pwd, data) = runtime.block_on(async {
        let addresses = dns::resolve_addresses(&config.get_remote().0).await;

        let (pwd, data) = tokio::join!(
            manager::connect(&config, &addresses),
            saml_server::start_server(),
        );

        (addresses, pwd, data)
    });

    let addr = addresses[0].to_string();
    let port = config.get_remote().1;

    let temp = tempfile::NamedTempFile::new().unwrap();
    config.save_config(temp.path());
    exec_ovpn_in_place(temp.path(), addr, port, &pwd, &data);
}
