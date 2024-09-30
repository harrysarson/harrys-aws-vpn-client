use crate::cmd::{kill_openvpn, ProcessInfo};
use std::sync::Arc;

pub struct OavcTask<T> {
    pub name: String,
    pub handle: tokio::task::JoinHandle<T>,
}

pub struct OavcProcessTask<T> {
    pub name: String,
    pub handle: tokio::task::JoinHandle<T>,
    pub info: Arc<ProcessInfo>,
}

impl<T> OavcTask<T> {
    pub fn abort(&self, log: bool) {
        self.handle.abort();
        if log {
            tracing::warn!("Stopped '{}'!", self.name);
        }
    }
}

impl<T> OavcProcessTask<T> {
    pub fn new(
        name: String,
        handle: tokio::task::JoinHandle<T>,
        info: Arc<ProcessInfo>,
    ) -> Self {
        Self {
            name,
            handle,
            info,
        }
    }

    pub fn abort(&self, log: bool) {
        self.handle.abort();
        {
            let pid = self.info.pid.lock().unwrap();

            if let Some(ref pid) = *pid {
                kill_openvpn(*pid);
            }

            if log {
                tracing::warn!("Stopped '{}' pid '{:?}'!", self.name, pid);
            }
        }
    }
}
