use crate::config::Config;
use crate::dns::DnsResolver;
use crate::manager::ConnectionManager;
use crate::task::{OavcProcessTask, OavcTask};
use std::sync::{Arc, Mutex};

pub struct VpnApp {
    pub config: Arc<Config>,
    pub server: Mutex<Option<OavcTask<()>>>,
    pub openvpn: Mutex<Option<OavcTask<()>>>,
    pub openvpn_connection: Arc<Mutex<Option<OavcProcessTask<i32>>>>,
    pub dns: Arc<DnsResolver>,
    pub connection_manager: Arc<Mutex<ConnectionManager>>,
}

impl VpnApp {
    pub fn new() -> VpnApp {
        let config = Arc::new(Config::new());
        VpnApp {
            config: config.clone(),
            server: Mutex::new(None),
            openvpn: Mutex::new(None),
            openvpn_connection: Arc::new(Mutex::new(None)),
            dns: Arc::new(DnsResolver::new(config)),
            connection_manager: Arc::new(Mutex::new(ConnectionManager::new())),
        }
    }

}

#[derive(Clone, Copy, Debug)]
pub enum State {
    Connecting,
    Connected,
    Disconnected,
}
