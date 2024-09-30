use crate::cmd::{connect_ovpn, ProcessInfo};
use crate::config::Pwd;
use crate::task::{OavcProcessTask, OavcTask};
use crate::VpnApp;
use std::collections::HashMap;
use std::sync::mpsc::SyncSender;
use std::sync::{Arc, Mutex};
use tokio::runtime::Runtime;
use warp::http::StatusCode;
use warp::reply::WithStatus;
use warp::{Filter, Rejection};

pub fn start_server(app: &Arc<VpnApp>, runtime: &Runtime) {
    tracing::info!("Starting SAML server at 0.0.0.0:35001...");
    let (tx, rx) = std::sync::mpsc::sync_channel::<Saml>(1);

    tracing::info!("Starting server");
    let sender = warp::any().map(move || tx.clone());

    let pwd = app.config.pwd.clone();
    let pwd = warp::any().map(move || pwd.clone());

    let saml = warp::post()
        .and(warp::body::form())
        .and(sender)
        .and(pwd)
        .and_then(
            move |data: HashMap<String, String>,
                  sender: SyncSender<Saml>,
                  pwd: Arc<Mutex<Option<Pwd>>>| {
                async move {
                    let pwd = pwd.lock().unwrap().as_ref().unwrap().pwd.clone();
                    let saml = Saml {
                        data: data["SAMLResponse"].clone(),
                        pwd,
                    };
                    sender.send(saml).unwrap();
                    println!("Got SAML data!");

                    Result::<WithStatus<_>, Rejection>::Ok(warp::reply::with_status(
                        "Got SAMLResponse field, it is now safe to close this window",
                        StatusCode::OK,
                    ))
                }
            },
        );

    let handle = runtime.spawn(warp::serve(saml).run(([0, 0, 0, 0], 35001)));

    let join = OavcTask {
        name: "SAML Server".to_string(),
        handle,
    };

    app.server.lock().unwrap().replace(join);
    let addr = app.config.addresses.clone();
    let st = app.openvpn_connection.clone();
    let manager = app.connection_manager.clone();

    loop {
        let data = rx.recv().unwrap();
        {
            tracing::info!("SAML Data: {:?}...", &data.data[..6]);
        }

        let addr = {
            let addr = addr.clone();
            let addr = addr.lock().unwrap();
            addr.as_ref().unwrap()[0].to_string()
        };
        let port = app.config.get_remote().1;

        let info = Arc::new(ProcessInfo::new());

        let handle = {
            let info = info.clone();
            let manager = manager.clone();
            let app = app.clone();
            let temp = tempfile::NamedTempFile::new().unwrap();
            app.config.save_config(temp.path());
            runtime.spawn(async move {
                let con = connect_ovpn(temp.path(), addr, port, data, info).await;
                let mut man = manager.lock().unwrap();
                man.try_disconnect(&app);
                con
            })
        };

        let task = OavcProcessTask::new("OpenVPN Connection".to_string(), handle, info);
        {
            let mut st = st.lock().unwrap();
            *st = Some(task);
        }

        manager.lock().unwrap().set_connected();
    }
}

#[derive(Debug, Clone)]
pub struct Saml {
    pub data: String,
    pub pwd: String,
}
