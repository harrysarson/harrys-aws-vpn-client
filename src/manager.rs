use crate::app::State;
use crate::cmd::run_ovpn;
use crate::config::Pwd;
use crate::task::OavcTask;
use crate::VpnApp;
use std::ops::Deref;
use std::sync::{Arc, Mutex, Weak};

pub struct ConnectionManager {
    pub app: Mutex<Weak<VpnApp>>,
    pub state: Arc<Mutex<State>>,
}

impl ConnectionManager {
    pub fn new() -> Self {
        Self {
            app: Mutex::new(Weak::new()),
            state:  Arc::new(Mutex::new(State::Disconnected)),
        }
    }

    pub fn set_app(&self, app: Arc<VpnApp>) {
        let mut l = self.app.lock().unwrap();
        *l = Arc::downgrade(&app);
    }

    pub fn change_connect_state(&self) {
        let state = {
            let state = { *(self.state.lock().unwrap()) };
            tracing::info!("Handling change... {:?}", &state);
            state
        };

        match state {
            State::Disconnected => self.connect(),
            State::Connected => self.disconnect(),
            State::Connecting => self.try_disconnect(),
        }
    }

    pub fn try_disconnect(&self) {
        let state = {
            let state = { *(self.state.lock().unwrap()) };
            tracing::info!("Handling disconnect... {:?}", &state);
            state
        };

        match state {
            State::Disconnected => (),
            _ => self.disconnect(),
        }
    }

    fn connect(&self) {
        tracing::info!("Connecting...");
        self.set_connecting();

        let (file, remote, addrs) = {
            let app = self.app.lock().unwrap();
            let app = app.upgrade().unwrap();

            (
                {
                    let x = app.config.config.lock().unwrap().deref().clone();
                    x
                },
                {
                    let x = app.config.remote.lock().unwrap().deref().clone();
                    x
                },
                {
                    let x = app.config.addresses.lock().unwrap().deref().clone();
                    x
                },
            )
        };

        if let Some(ref addrs) = addrs {
            if let Some(ref remote) = remote {
                if let Some(ref file) = file {
                    let first_addr = addrs[0].to_string();
                    let config_file = file.clone();
                    let port = remote.1;

                    let pwd = {
                        let app = self.app.lock().unwrap();
                        let app = app.upgrade().unwrap();
                        app.config.pwd.clone()
                    };

                    let join = {
                        let app = self.app.lock().unwrap();
                        let app = app.upgrade().unwrap();

                        app.runtime.spawn(async move {
                            let mut lock = pwd.lock().await;
                            let auth = run_ovpn(config_file, first_addr, port).await; // Failure point addrs[0]
                            *lock = Some(Pwd { pwd: auth.pwd });

                            open::that(auth.url).unwrap()
                        })
                    };

                    let app = self.app.lock().unwrap();
                    let app = app.upgrade().unwrap();
                    app.openvpn.lock().unwrap().replace(OavcTask {
                        name: "OpenVPN Initial SAML Process".to_string(),
                        handle: join,
                    });
                }
                return;
            }
        }

        tracing::error!("No file selected");
    }

    pub fn force_disconnect(&self) {
        tracing::warn!("Forcing disconnect...");

        let app = self.app.lock().unwrap();
        let app = app.upgrade().unwrap();
        let mut openvpn = app.openvpn.lock().unwrap();

        if let Some(ref srv) = openvpn.take() {
            srv.abort(false);
        }

        let openvpn_connection = app.openvpn_connection.clone();
        let mut openvpn_connection = openvpn_connection.lock().unwrap();
        if let Some(ref conn) = openvpn_connection.take() {
            conn.abort(false);
        }
    }

    fn disconnect(&self) {
        tracing::info!("Disconnecting...");

        self.set_disconnected();

        {
            let app = self.app.lock().unwrap();
            let app = app.upgrade().unwrap();

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

    fn set_connecting(&self) {
        *self.state.lock().unwrap() = State::Connecting;
    }

    pub fn set_connected(&self) {
        *self.state.lock().unwrap() = State::Connected;
    }

    fn set_disconnected(&self) {
        *self.state.lock().unwrap() = State::Disconnected;
    }
}
