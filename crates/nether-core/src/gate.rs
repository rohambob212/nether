//! Local TCP gate in front of the Aether SOCKS listener.
//!
//! With "always-on core" the tunnel engine runs continuously; connecting and
//! disconnecting only opens and closes this gate, so reconnects are instant
//! and the core's handshake stays warm.
//!
//! ponytail ceilings: (1) the core lives as long as the app process — surviving
//! app restarts would need an OS service wrapper; (2) connections accepted in
//! the microseconds between `accept()` and registration may outlive a `close()`
//! until their transfer ends — harmless on localhost.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub struct ProxyGate {
    conns: Arc<Mutex<HashMap<u64, tokio::task::JoinHandle<()>>>>,
    accept_task: Mutex<Option<tokio::task::JoinHandle<()>>>,
    open: Arc<AtomicBool>,
    next_id: Arc<AtomicU64>,
}

impl Default for ProxyGate {
    fn default() -> Self {
        Self::new()
    }
}

impl ProxyGate {
    pub fn new() -> Self {
        Self {
            conns: Arc::new(Mutex::new(HashMap::new())),
            accept_task: Mutex::new(None),
            open: Arc::new(AtomicBool::new(false)),
            next_id: Arc::new(AtomicU64::new(1)),
        }
    }

    pub fn is_open(&self) -> bool {
        self.open.load(Ordering::Relaxed)
    }

    /// Bind the user-facing SOCKS address and splice every connection through
    /// to the engine's internal listener. Returns the bound port (useful when
    /// callers pass 0).
    pub fn open(&self, bind_host: &str, port: u16, upstream: (String, u16)) -> Result<u16, String> {
        if self.is_open() {
            return Err("gate already open".into());
        }
        let std_listener = std::net::TcpListener::bind((bind_host, port))
            .map_err(|e| format!("bind {bind_host}:{port}: {e}"))?;
        std_listener
            .set_nonblocking(true)
            .map_err(|e| format!("gate listener: {e}"))?;
        let listener = tokio::net::TcpListener::from_std(std_listener)
            .map_err(|e| format!("gate listener: {e}"))?;

        let conns = self.conns.clone();
        let open = self.open.clone();
        let next_id = self.next_id.clone();
        let bound_port = listener.local_addr().map(|a| a.port()).unwrap_or(port);
        let log_upstream = upstream.clone();
        open.store(true, Ordering::Relaxed);

        let task = tokio::spawn(async move {
            while let Ok((client, _)) = listener.accept().await {
                let id = next_id.fetch_add(1, Ordering::Relaxed);
                let handle =
                    tokio::spawn(splice(conns.clone(), id, client, upstream.clone()));
                conns.lock().unwrap().insert(id, handle);
            }
            open.store(false, Ordering::Relaxed);
        });
        *self.accept_task.lock().unwrap() = Some(task);
        log::info!(
            "[nether] gate open on {bind_host}:{port} -> {}:{}",
            log_upstream.0,
            log_upstream.1
        );
        Ok(bound_port)
    }

    /// Close the listener and tear down every live spliced connection. The
    /// engine behind it keeps running.
    pub fn close(&self) {
        if let Some(task) = self.accept_task.lock().unwrap().take() {
            task.abort();
        }
        for (_, handle) in self.conns.lock().unwrap().drain() {
            handle.abort();
        }
        self.open.store(false, Ordering::Relaxed);
        log::info!("[nether] gate closed");
    }
}

async fn splice(
    conns: Arc<Mutex<HashMap<u64, tokio::task::JoinHandle<()>>>>,
    id: u64,
    mut client: tokio::net::TcpStream,
    upstream: (String, u16),
) {
    let _cleanup = Cleanup {
        conns: conns.clone(),
        id,
    };
    let connect = tokio::net::TcpStream::connect((upstream.0.as_str(), upstream.1));
    let mut server = match tokio::time::timeout(Duration::from_secs(5), connect).await {
        Ok(Ok(s)) => s,
        _ => {
            log::warn!("[nether] gate: upstream connect failed");
            return;
        }
    };
    if tokio::io::copy_bidirectional(&mut client, &mut server)
        .await
        .is_err()
    {
        log::debug!("[nether] gate: connection ended with error");
    }
}

struct Cleanup {
    conns: Arc<Mutex<HashMap<u64, tokio::task::JoinHandle<()>>>>,
    id: u64,
}

impl Drop for Cleanup {
    fn drop(&mut self) {
        self.conns.lock().unwrap().remove(&self.id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    #[tokio::test]
    async fn gate_splices_traffic_and_close_refuses_new_connections() {
        // Minimal echo "engine" behind the gate.
        let upstream = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let up_port = upstream.local_addr().unwrap().port();
        tokio::spawn(async move {
            while let Ok((mut s, _)) = upstream.accept().await {
                tokio::spawn(async move {
                    let mut buf = [0u8; 64];
                    if let Ok(n) = s.read(&mut buf).await {
                        let _ = s.write_all(&buf[..n]).await;
                    }
                });
            }
        });

        let gate = ProxyGate::new();
        let bound = gate
            .open("127.0.0.1", 0, ("127.0.0.1".into(), up_port))
            .unwrap();
        assert!(gate.is_open());

        let mut client = tokio::net::TcpStream::connect(("127.0.0.1", bound))
            .await
            .unwrap();
        client.write_all(b"ping").await.unwrap();
        let mut buf = [0u8; 4];
        client.read_exact(&mut buf).await.unwrap();
        assert_eq!(&buf, b"ping");

        gate.close();
        assert!(!gate.is_open());
        assert!(
            tokio::net::TcpStream::connect(("127.0.0.1", bound))
                .await
                .is_err(),
            "closed gate must refuse new connections"
        );
    }
}

