use crate::cmd::run_ovpn;
use crate::config::{Config, Pwd};
use std::net::IpAddr;



pub(crate) async fn connect<'scope>(config: &Config, addrs: &[IpAddr]) -> Pwd {

    tracing::info!("Connecting...");

    let remote = config.get_remote();

    let first_addr = addrs[0].to_string();
    let port = remote.1;

    let temp = tempfile::NamedTempFile::new().unwrap();
    config.save_config(temp.path());

    let auth = run_ovpn(temp.path(), first_addr, port).await;

    open::that(auth.url).unwrap();

    Pwd::new(auth.pwd)
}
