use std::collections::HashMap;
use std::net::SocketAddr;
use tracing::Instrument;
use warp::http::StatusCode;
use warp::reply::WithStatus;
use warp::{Filter, Rejection};

pub(crate) async fn start_server() -> Saml {
    tracing::info!("Starting SAML server at 0.0.0.0:35001...");
    let (tx, mut rx) = tokio::sync::mpsc::channel::<_>(1);

    let (server_shutdown_tx, server_shutdown_rx) = tokio::sync::oneshot::channel();

    tracing::info!("Starting server");


    let saml =
        warp::post()
            .and(warp::body::form())
            .and_then(move |data: HashMap<String, String>| {
                let sender = tx.clone();
                {
                    async move {
                        let saml = Saml {
                            data: data["SAMLResponse"].clone(),
                        };
                        sender.send(saml).await.unwrap();
                        println!("Got SAML data!");

                        Result::<WithStatus<_>, Rejection>::Ok(warp::reply::with_status(
                            "Got SAMLResponse field, it is now safe to close this window",
                            StatusCode::OK,
                        ))
                    }
                }
            });

    let addr: SocketAddr = ([0, 0, 0, 0], 35001).into();

    let (_, fut) = warp::serve(saml).bind_with_graceful_shutdown(addr, async move {
        server_shutdown_rx.await.unwrap();
    });
    let span = tracing::info_span!("Server::run", ?addr);
    tracing::info!(parent: &span, "listening on http://{}", addr);

    let get_data = async {
        // There must be data in this channel
        let data = rx.recv().await.unwrap();

        tracing::info!("SAML Data: {:?}...", &data.data[..6]);

        server_shutdown_tx.send(()).unwrap();
        data
    };

    tokio::join!(fut.instrument(span), get_data).1
}

#[derive(Debug, Clone)]
pub(crate) struct Saml {
    pub(crate) data: String,
}
