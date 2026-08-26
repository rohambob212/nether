//! Device-wide VPN mode: turn IP packets from a TUN file descriptor into
//! SOCKS5 connections against the local Aether proxy.
//!
//! Android's `VpnService` hands us an already-configured TUN fd, so this module
//! never creates or routes an interface itself — it only owns the packets. That
//! also keeps it buildable (and testable) on plain Linux, where a fd from
//! `/dev/net/tun` behaves identically.
//!
//! The loop that matters:
//!
//! ```text
//!   TUN fd ──► ipstack ──► per-flow stream ──► SOCKS5 ──► 127.0.0.1:1819
//! ```
//!
//! Nothing here prevents a routing loop — that is the caller's job. On Android
//! the service excludes our own package from the tunnel, so Aether's sockets
//! reach the network directly instead of being fed back into this stack.

use std::io;
use std::net::SocketAddr;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd, RawFd};
use std::pin::Pin;
use std::task::{Context, Poll};

use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, ReadBuf};
use tokio::io::unix::AsyncFd;

const SOCKS5: u8 = 0x05;
const NO_AUTH: u8 = 0x00;
const CMD_CONNECT: u8 = 0x01;
const CMD_UDP_ASSOCIATE: u8 = 0x03;
const ATYP_V4: u8 = 0x01;
const ATYP_V6: u8 = 0x04;
const RSV: u8 = 0x00;

/// Largest datagram we will relay in either direction.
const UDP_BUF: usize = 65_535;

// ---------------------------------------------------------------------------
// TUN device
// ---------------------------------------------------------------------------

/// Async wrapper over a TUN file descriptor.
///
/// `VpnService.establish()` returns a blocking fd; readiness-based IO needs it
/// non-blocking, so we flip the flag on take-over. The fd is owned from here on
/// and closed when this value drops.
struct TunDevice {
    inner: AsyncFd<OwnedFd>,
}

impl TunDevice {
    /// # Safety
    /// `fd` must be a valid, exclusively-owned TUN file descriptor.
    unsafe fn from_raw_fd(fd: RawFd) -> io::Result<Self> {
        let flags = libc::fcntl(fd, libc::F_GETFL);
        if flags < 0 {
            return Err(io::Error::last_os_error());
        }
        if libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) < 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(Self {
            inner: AsyncFd::new(OwnedFd::from_raw_fd(fd))?,
        })
    }
}

impl AsyncRead for TunDevice {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        loop {
            let mut guard = match self.inner.poll_read_ready(cx) {
                Poll::Ready(Ok(g)) => g,
                Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
                Poll::Pending => return Poll::Pending,
            };
            let unfilled = buf.initialize_unfilled();
            let read = guard.try_io(|fd| {
                let n = unsafe {
                    libc::read(
                        fd.get_ref().as_raw_fd(),
                        unfilled.as_mut_ptr().cast(),
                        unfilled.len(),
                    )
                };
                if n < 0 {
                    Err(io::Error::last_os_error())
                } else {
                    Ok(n as usize)
                }
            });
            match read {
                Ok(Ok(n)) => {
                    buf.advance(n);
                    return Poll::Ready(Ok(()));
                }
                Ok(Err(e)) => return Poll::Ready(Err(e)),
                // Readiness was stale; the guard is cleared, so poll again.
                Err(_would_block) => continue,
            }
        }
    }
}

impl AsyncWrite for TunDevice {
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        loop {
            let mut guard = match self.inner.poll_write_ready(cx) {
                Poll::Ready(Ok(g)) => g,
                Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
                Poll::Pending => return Poll::Pending,
            };
            let written = guard.try_io(|fd| {
                let n =
                    unsafe { libc::write(fd.get_ref().as_raw_fd(), buf.as_ptr().cast(), buf.len()) };
                if n < 0 {
                    Err(io::Error::last_os_error())
                } else {
                    Ok(n as usize)
                }
            });
            match written {
                Ok(Ok(n)) => return Poll::Ready(Ok(n)),
                Ok(Err(e)) => return Poll::Ready(Err(e)),
                Err(_would_block) => continue,
            }
        }
    }

    // A TUN fd has no userspace buffer to drain and cannot be half-closed.
    fn poll_flush(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }

    fn poll_shutdown(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}

// ---------------------------------------------------------------------------
// SOCKS5 client
// ---------------------------------------------------------------------------

/// Encode an address as a SOCKS5 ATYP + address + port triplet.
fn encode_addr(out: &mut Vec<u8>, addr: SocketAddr) {
    match addr {
        SocketAddr::V4(v4) => {
            out.push(ATYP_V4);
            out.extend_from_slice(&v4.ip().octets());
        }
        SocketAddr::V6(v6) => {
            out.push(ATYP_V6);
            out.extend_from_slice(&v6.ip().octets());
        }
    }
    out.extend_from_slice(&addr.port().to_be_bytes());
}

/// Read a SOCKS5 reply and return its BND.ADDR:BND.PORT.
async fn read_reply<S: AsyncRead + Unpin>(sock: &mut S) -> io::Result<SocketAddr> {
    let mut head = [0u8; 4];
    sock.read_exact(&mut head).await?;
    if head[0] != SOCKS5 {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "not socks5"));
    }
    if head[1] != 0 {
        return Err(io::Error::other(format!("socks5 refused (rep {})", head[1])));
    }
    let ip = match head[3] {
        ATYP_V4 => {
            let mut a = [0u8; 4];
            sock.read_exact(&mut a).await?;
            std::net::IpAddr::from(a)
        }
        ATYP_V6 => {
            let mut a = [0u8; 16];
            sock.read_exact(&mut a).await?;
            std::net::IpAddr::from(a)
        }
        // ATYP_DOMAIN: a proxy is allowed to answer with a name, but we have
        // nothing to resolve it with down here, so treat it as unusable.
        other => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("unsupported socks5 atyp {other}"),
            ))
        }
    };
    let mut port = [0u8; 2];
    sock.read_exact(&mut port).await?;
    Ok(SocketAddr::new(ip, u16::from_be_bytes(port)))
}

/// Complete the SOCKS5 greeting (no authentication).
async fn greet<S: AsyncRead + AsyncWrite + Unpin>(sock: &mut S) -> io::Result<()> {
    sock.write_all(&[SOCKS5, 1, NO_AUTH]).await?;
    let mut resp = [0u8; 2];
    sock.read_exact(&mut resp).await?;
    if resp != [SOCKS5, NO_AUTH] {
        return Err(io::Error::other("socks5 rejected no-auth"));
    }
    Ok(())
}

/// Open a proxied TCP connection to `dest`.
async fn socks_connect(proxy: SocketAddr, dest: SocketAddr) -> io::Result<tokio::net::TcpStream> {
    let mut sock = tokio::net::TcpStream::connect(proxy).await?;
    sock.set_nodelay(true).ok();
    greet(&mut sock).await?;

    let mut req = vec![SOCKS5, CMD_CONNECT, RSV];
    encode_addr(&mut req, dest);
    sock.write_all(&req).await?;
    read_reply(&mut sock).await?;
    Ok(sock)
}

/// Open a SOCKS5 UDP association.
///
/// Returns the bound relay socket plus the TCP control connection, which must
/// be held for as long as the association is in use — dropping it tells the
/// proxy to tear the association down.
async fn socks_udp_associate(
    proxy: SocketAddr,
) -> io::Result<(tokio::net::UdpSocket, tokio::net::TcpStream)> {
    let mut ctrl = tokio::net::TcpStream::connect(proxy).await?;
    greet(&mut ctrl).await?;

    // We do not know our source port until the OS assigns one, and Aether does
    // not enforce the client address, so request the wildcard.
    let unspecified: SocketAddr = if proxy.is_ipv4() {
        "0.0.0.0:0".parse().unwrap()
    } else {
        "[::]:0".parse().unwrap()
    };
    let mut req = vec![SOCKS5, CMD_UDP_ASSOCIATE, RSV];
    encode_addr(&mut req, unspecified);
    ctrl.write_all(&req).await?;

    let mut relay = read_reply(&mut ctrl).await?;
    // A wildcard BND.ADDR means "same host as the control connection".
    if relay.ip().is_unspecified() {
        relay.set_ip(proxy.ip());
    }

    let bind: SocketAddr = if relay.is_ipv4() {
        "0.0.0.0:0".parse().unwrap()
    } else {
        "[::]:0".parse().unwrap()
    };
    let sock = tokio::net::UdpSocket::bind(bind).await?;
    sock.connect(relay).await?;
    Ok((sock, ctrl))
}

/// Wrap a payload in a SOCKS5 UDP request header addressed to `dest`.
fn encode_udp(dest: SocketAddr, payload: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(payload.len() + 22);
    out.extend_from_slice(&[RSV, RSV, 0x00]); // RSV RSV FRAG
    encode_addr(&mut out, dest);
    out.extend_from_slice(payload);
    out
}

/// Strip a SOCKS5 UDP response header, returning the payload offset.
fn udp_payload_offset(buf: &[u8]) -> Option<usize> {
    if buf.len() < 4 || buf[2] != 0x00 {
        return None; // too short, or a fragment we do not reassemble
    }
    let addr_len = match buf[3] {
        ATYP_V4 => 4,
        ATYP_V6 => 16,
        0x03 => 1 + *buf.get(4)? as usize,
        _ => return None,
    };
    let offset = 4 + addr_len + 2;
    (buf.len() >= offset).then_some(offset)
}

// ---------------------------------------------------------------------------
// Flow handling
// ---------------------------------------------------------------------------

async fn handle_tcp(
    mut stream: ipstack::IpStackTcpStream,
    proxy: SocketAddr,
) -> io::Result<()> {
    let dest = stream.peer_addr();
    let mut upstream = socks_connect(proxy, dest).await?;
    tokio::io::copy_bidirectional(&mut stream, &mut upstream).await?;
    Ok(())
}

async fn handle_udp(
    mut stream: ipstack::IpStackUdpStream,
    proxy: SocketAddr,
) -> io::Result<()> {
    let dest = stream.peer_addr();
    // ponytail: one association per flow. DNS makes a new flow per query, so a
    // busy device opens a lot of short-lived control connections. Pool them by
    // source address if that ever shows up in a profile.
    let (relay, _ctrl) = socks_udp_associate(proxy).await?;

    let mut from_tun = vec![0u8; UDP_BUF];
    let mut from_relay = vec![0u8; UDP_BUF];
    loop {
        tokio::select! {
            read = stream.read(&mut from_tun) => {
                let n = read?;
                if n == 0 {
                    return Ok(());
                }
                relay.send(&encode_udp(dest, &from_tun[..n])).await?;
            }
            read = relay.recv(&mut from_relay) => {
                let n = read?;
                match udp_payload_offset(&from_relay[..n]) {
                    Some(off) => stream.write_all(&from_relay[off..n]).await?,
                    None => log::debug!("[vpn] dropped malformed udp reply"),
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

/// Pump the TUN device until the returned task is aborted or the fd closes.
///
/// Takes ownership of `tun_fd`.
///
/// # Safety
/// `tun_fd` must be a valid, exclusively-owned TUN file descriptor.
pub async unsafe fn run(tun_fd: RawFd, proxy: SocketAddr, mtu: u16) -> Result<(), String> {
    let device = TunDevice::from_raw_fd(tun_fd).map_err(|e| format!("tun fd: {e}"))?;

    let mut config = ipstack::IpStackConfig::default();
    config.mtu_unchecked(mtu);
    // Android hands over a bare IP tun with no packet-information prefix.
    config.packet_information(false);

    let mut stack = ipstack::IpStack::new(config, device);
    log::info!("[vpn] tunnelling device traffic to socks5 {proxy} (mtu {mtu})");

    loop {
        let stream = match stack.accept().await {
            Ok(s) => s,
            Err(e) => {
                log::error!("[vpn] stack stopped: {e}");
                return Err(e.to_string());
            }
        };
        match stream {
            ipstack::IpStackStream::Tcp(tcp) => {
                let dest = tcp.peer_addr();
                tokio::spawn(async move {
                    if let Err(e) = handle_tcp(tcp, proxy).await {
                        log::debug!("[vpn] tcp {dest} ended: {e}");
                    }
                });
            }
            ipstack::IpStackStream::Udp(udp) => {
                let dest = udp.peer_addr();
                tokio::spawn(async move {
                    if let Err(e) = handle_udp(udp, proxy).await {
                        log::debug!("[vpn] udp {dest} ended: {e}");
                    }
                });
            }
            // ICMP and friends: the tunnel is a SOCKS proxy, it has nowhere to
            // put a raw IP packet. Dropping is the honest answer.
            ipstack::IpStackStream::UnknownTransport(u) => {
                log::debug!("[vpn] dropped {:?} packet", u.ip_protocol());
            }
            ipstack::IpStackStream::UnknownNetwork(p) => {
                log::debug!("[vpn] dropped {} byte non-ip packet", p.len());
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::AsyncReadExt;

    #[test]
    fn udp_header_roundtrip() {
        let dest: SocketAddr = "1.1.1.1:53".parse().unwrap();
        let framed = encode_udp(dest, b"query");
        // RSV RSV FRAG ATYP + 4 byte v4 + 2 byte port
        assert_eq!(&framed[..4], &[0x00, 0x00, 0x00, ATYP_V4]);
        assert_eq!(&framed[4..8], &[1, 1, 1, 1]);
        assert_eq!(&framed[8..10], &53u16.to_be_bytes());

        let off = udp_payload_offset(&framed).expect("header parses");
        assert_eq!(&framed[off..], b"query");
    }

    #[test]
    fn udp_offset_rejects_fragments_and_runts() {
        assert!(udp_payload_offset(&[0, 0]).is_none(), "runt");
        // FRAG != 0 means a fragmented datagram, which we do not reassemble.
        assert!(udp_payload_offset(&[0, 0, 1, ATYP_V4, 1, 1, 1, 1, 0, 53]).is_none());
        // Truncated address must not slice past the end.
        assert!(udp_payload_offset(&[0, 0, 0, ATYP_V4, 1, 1]).is_none());
    }

    #[test]
    fn v6_addresses_encode_as_atyp_4() {
        let mut out = Vec::new();
        encode_addr(&mut out, "[2606:4700::1111]:443".parse().unwrap());
        assert_eq!(out[0], ATYP_V6);
        assert_eq!(out.len(), 1 + 16 + 2);
        assert_eq!(&out[17..], &443u16.to_be_bytes());
    }

    /// The handshake is the part a proxy will reject silently if we get a byte
    /// wrong, so drive it against a scripted server.
    #[tokio::test]
    async fn connect_speaks_the_expected_handshake() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let proxy = listener.local_addr().unwrap();

        let server = tokio::spawn(async move {
            let (mut sock, _) = listener.accept().await.unwrap();
            let mut greeting = [0u8; 3];
            sock.read_exact(&mut greeting).await.unwrap();
            assert_eq!(greeting, [SOCKS5, 1, NO_AUTH]);
            sock.write_all(&[SOCKS5, NO_AUTH]).await.unwrap();

            let mut req = [0u8; 10];
            sock.read_exact(&mut req).await.unwrap();
            assert_eq!(&req[..4], &[SOCKS5, CMD_CONNECT, RSV, ATYP_V4]);
            assert_eq!(&req[4..8], &[93, 184, 216, 34]);
            assert_eq!(&req[8..], &443u16.to_be_bytes());

            sock.write_all(&[SOCKS5, 0, RSV, ATYP_V4, 0, 0, 0, 0, 0, 0])
                .await
                .unwrap();
        });

        socks_connect(proxy, "93.184.216.34:443".parse().unwrap())
            .await
            .expect("handshake completes");
        server.await.unwrap();
    }

    #[tokio::test]
    async fn connect_surfaces_a_refusal() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let proxy = listener.local_addr().unwrap();
        tokio::spawn(async move {
            let (mut sock, _) = listener.accept().await.unwrap();
            let mut buf = [0u8; 3];
            sock.read_exact(&mut buf).await.unwrap();
            sock.write_all(&[SOCKS5, NO_AUTH]).await.unwrap();
            let mut req = [0u8; 10];
            sock.read_exact(&mut req).await.unwrap();
            // REP 0x05 = connection refused by destination.
            sock.write_all(&[SOCKS5, 5, RSV, ATYP_V4, 0, 0, 0, 0, 0, 0])
                .await
                .unwrap();
        });

        let err = socks_connect(proxy, "93.184.216.34:443".parse().unwrap())
            .await
            .expect_err("refusal must not look like success");
        assert!(err.to_string().contains("rep 5"), "got: {err}");
    }
}
