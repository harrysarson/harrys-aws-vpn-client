use crate::config::Config;
use crate::dns::DnsResolver;
use crate::manager::ConnectionManager;
use crate::task::{OavcProcessTask, OavcTask};
use std::sync::{Arc, Mutex};
use tokio::runtime::Runtime;

pub struct VpnApp {
    pub config: Arc<Config>,
    pub server: Mutex<Option<OavcTask<()>>>,
    pub openvpn: Mutex<Option<OavcTask<()>>>,
    pub openvpn_connection: Arc<Mutex<Option<OavcProcessTask<i32>>>>,
    pub runtime: Arc<Runtime>,
    pub dns: Arc<DnsResolver>,
    pub connection_manager: Arc<Mutex<ConnectionManager>>,
}

impl VpnApp {
    pub fn new() -> VpnApp {
        let config = Arc::new(Config::new());
        let runtime = Arc::new(
            tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .unwrap(),
        );
        VpnApp {
            config: config.clone(),
            server: Mutex::new(None),
            openvpn: Mutex::new(None),
            openvpn_connection: Arc::new(Mutex::new(None)),
            runtime: runtime.clone(),
            dns: Arc::new(DnsResolver::new(config, runtime)),
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
