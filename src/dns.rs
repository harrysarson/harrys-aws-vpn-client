use domain::base::iana::Class;
use domain::base::{Dname, Rtype};
use domain::rdata::A;
use rand::prelude::*;
use std::net::IpAddr;
use std::str::FromStr;

async fn resolve_to_ip_list(remote: &str) -> Vec<IpAddr> {
    tracing::info!("Looking up into '{}'...", remote);

    let resolver = domain::resolv::StubResolver::new();
    let d: domain::base::Dname<Vec<u8>> = Dname::from_str(remote).unwrap();
    let r = resolver.query((d, Rtype::A, Class::In)).await.unwrap();

    let msg = r.into_message();
    let ans = msg.answer().unwrap().limit_to::<A>();

    ans.filter_map(|v| {
        let v = v.ok()?;
        let v = IpAddr::V4(v.into_data().addr());
        tracing::info!("Resolved '{}'.", v);
        Some(v)
    })
    .collect::<Vec<_>>()
}

pub(crate) async fn resolve_addresses(remote: &str) -> Vec<IpAddr> {
    let random_start = rng_domain();
    let remote_with_rng_domain = format!("{random_start}.{remote}");

    let mut all = resolve_to_ip_list(&remote_with_rng_domain).await;
    if all.is_empty() {
        tracing::warn!(
            "Unable to resolve any addresses at '{}'.",
            remote_with_rng_domain
        );
        tracing::warn!("Attempting to resolve without any randomized domain...");
        all = resolve_to_ip_list(remote).await;
    };

    all
}

fn rng_domain() -> String {
    let mut rng = thread_rng();
    let mut bts = [0u8; 12];
    rng.fill_bytes(&mut bts);
    hex::encode(bts)
}
