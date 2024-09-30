use crate::app::State;
use crate::cmd::run_ovpn;
use crate::config::{Pwd};
use crate::VpnApp;
use std::ops::Deref;
use std::thread::Scope;

pub struct ConnectionManager {
    pub state: State,
}

impl ConnectionManager {
    pub fn new() -> Self {
        Self {
            state: State::Disconnected,
        }
    }

    pub fn change_connect_state<'scope>(&mut self, s: &'scope Scope<'scope, '_>, app: &VpnApp) {
        tracing::info!("Handling change... {:?}", &self.state);

        match self.state {
            State::Disconnected => self.connect(s, app),
            State::Connected => self.disconnect(app),
            State::Connecting => self.try_disconnect(app),
        }
    }

    pub fn try_disconnect(&mut self, app: &VpnApp) {
        tracing::info!("Handling disconnect... {:?}", &self.state);

        match self.state {
            State::Disconnected => (),
            _ => self.disconnect(app),
        }
    }

    fn connect<'scope>(&mut self, s: &'scope Scope<'scope, '_>, app: &VpnApp) {
        tracing::info!("Connecting...");
        self.set_connecting();

        let (addrs,) = ({
            let x = app.config.addresses.lock().unwrap().deref().clone();
            x
        },);

        let remote = app.config.get_remote();

        if let Some(ref addrs) = addrs {
            let first_addr = addrs[0].to_string();
            let port = remote.1;

            let pwd = { app.config.pwd.clone() };

            let temp = tempfile::NamedTempFile::new().unwrap();
            app.config.save_config(temp.path());

            s.spawn(move || {
                let mut lock = pwd.lock().unwrap();
                let auth = run_ovpn(temp.path(), first_addr, port);
                *lock = Some(Pwd { pwd: auth.pwd });

                open::that(auth.url).unwrap();
            });

            return;
        }

        tracing::error!("No file selected");
    }

    pub fn force_disconnect(&mut self, app: &VpnApp) {
        tracing::warn!("Forcing disconnect...");

        self.disconnect(app);
    }

    fn disconnect(&mut self, app: &VpnApp) {
        tracing::info!("Disconnecting...");

        self.set_disconnected();

        {
            let mut openvpn = app.openvpn.lock().unwrap();

            if let Some(ref srv) = openvpn.take() {
                srv.abort(true);
                tracing::info!("OpenVPN Auth Disconnected!");
            }

            let openvpn_connection = app.openvpn_connection.clone();
            let mut openvpn_connection = openvpn_connection.lock().unwrap();
            if let Some(ref conn) = openvpn_connection.take() {
                conn.abort(true);
                tracing::info!("OpenVPN disconnected!");
            }

            tracing::info!("Disconnected!");
        }
    }

    fn set_connecting(&mut self) {
        self.state = State::Connecting;
    }

    pub fn set_connected(&mut self) {
        self.state = State::Connected;
    }

    fn set_disconnected(&mut self) {
        self.state = State::Disconnected;
    }
}
