#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::similar_names)]
#![allow(clippy::struct_field_names)]


mod app;
mod cmd;
mod config;
mod dns;
mod local_config;
mod manager;
mod saml_server;
mod storage;
mod task;

use dns::DnsResolver;
use tokio::runtime::Runtime;

use crate::app::VpnApp;
use crate::cmd::kill_openvpn;
use crate::local_config::LocalConfig;
use std::path::Path;
use std::sync::Arc;
use std::thread;
use std::time::Duration;

fn main() {
    tracing_subscriber::fmt::init();

    let runtime =
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .unwrap();

    let vpn_app = Arc::new(VpnApp::new());

    thread::scope(|s| {
        tracing::debug!("In main() scope");
        s.spawn(|| saml_server::start_server(&vpn_app, &runtime));

        let vpn_app = vpn_app.clone();
        build_main_grid(&vpn_app, &runtime);

        if let Some(p) = LocalConfig::read_last_pid() {
            tracing::warn!("[{p}] Last OpenVPN session was not closed properly...");
            tracing::warn!("[{p}] Asking to kill it in 5 seconds...");
            s.spawn(move || {
                std::thread::sleep(Duration::from_secs(5));
                kill_openvpn(p);
            });
        }

        let mut  manager = vpn_app.connection_manager.lock().unwrap();
        manager.change_connect_state(s, &vpn_app);
    });

    let mut manager = vpn_app.connection_manager.lock().unwrap();
    manager.force_disconnect(&vpn_app);

}

fn build_main_grid(app: &VpnApp, runtime: &Runtime) {
    if let Some(c) = LocalConfig::read_last_file() {
        set_file(c, app, &app.dns, runtime);
    }
}

fn set_file(path: impl AsRef<Path>, app: &VpnApp, dns: &DnsResolver, runtime: &Runtime) {
    let path = path.as_ref();
    tracing::info!("Saving config to {path:?}");
    LocalConfig::save_last_file(path);

    *app.config.contents.lock().unwrap() = std::fs::read_to_string(path).unwrap();

    dns.resolve_addresses(runtime);
}
