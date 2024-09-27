use crate::config::Config;
use domain::base::iana::Class;
use domain::base::{Dname, Rtype};
use domain::rdata::A;
use rand::prelude::*;
use std::net::IpAddr;
use std::ops::Deref;
use std::str::FromStr;
use std::sync::Arc;
use tokio::runtime::Runtime;

pub struct DnsResolver {
    pub config: Arc<Config>,
    pub runtime: Arc<Runtime>,
}

impl DnsResolver {
    pub fn new(config: Arc<Config>, runtime: Arc<Runtime>) -> Self {
        Self {
            config,
            runtime,
        }
    }

    fn resolve_to_ip_list(&self, remote: String) -> Vec<IpAddr> {
        tracing::info!("Looking up into '{}'...", remote);

        let resolver = domain::resolv::StubResolver::new();
        let d: domain::base::Dname<Vec<u8>> = Dname::from_str(&remote).unwrap();
        let r = self
            .runtime
            .block_on(async { resolver.query((d, Rtype::A, Class::In)).await })
            .unwrap();

        let msg = r.into_message();
        let ans = msg.answer().unwrap().limit_to::<A>();
        let all = ans
            .filter(|v| v.is_ok())
            .map(|v| v.unwrap())
            .map(|v| v.into_data())
            .map(|v| v.addr())
            .map(|v| IpAddr::V4(v))
            .inspect(|v| tracing::info!("Resolved '{}'.", v))
            .collect::<Vec<_>>();
        all
    }


    pub fn resolve_addresses(&self) {
        let remote = self.config.remote.lock().unwrap().deref().clone().unwrap();

        let random_start = rng_domain();
        let remote_with_rng_domain = format!("{}.{}", random_start, remote.0);


        let mut all = self.resolve_to_ip_list(remote_with_rng_domain.clone());
        if all.is_empty() {
            tracing::warn!("Unable to resolve any addresses at '{}'.", remote_with_rng_domain);
            tracing::warn!("Attempting to resolve without any randomized domain...");
            all = self.resolve_to_ip_list(remote.0);
        };


        let mut br = self.config.addresses.lock().unwrap();
        *br = Some(all);
    }
}

fn rng_domain() -> String {
    let mut rng = thread_rng();
    let mut bts = [0u8; 12];
    rng.fill_bytes(&mut bts);
    hex::encode(bts)
}
