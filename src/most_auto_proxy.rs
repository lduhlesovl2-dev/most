use hbb_common::{config::{Config, Socks5Server}, log, tokio};
use std::{io, time::Duration};
use tokio::{io::{AsyncReadExt, AsyncWriteExt}, net::TcpStream, time::{sleep, timeout}};

const PROXY: &str = "127.0.0.1:10808";

fn managed(proxy: Option<&str>) -> bool {
    matches!(proxy, None | Some("127.0.0.1:10808"))
}

async fn probe_proxy_at(address: &str) -> io::Result<()> {
    let mut stream = TcpStream::connect(address).await?;
    stream.write_all(&[5, 1, 0]).await?;
    let mut greeting = [0; 2];
    stream.read_exact(&mut greeting).await?;
    if greeting != [5, 0] {
        return Err(io::Error::new(io::ErrorKind::Other, "SOCKS authentication unavailable"));
    }
    // Probe an actual outbound TCP route; a listening HAPP port alone is insufficient.
    let host = b"ovh-fr2.rustdesk.com";
    let mut request = vec![5, 1, 0, 3, host.len() as u8];
    request.extend_from_slice(host);
    request.extend_from_slice(&21117u16.to_be_bytes());
    stream.write_all(&request).await?;
    let mut reply = [0; 4];
    stream.read_exact(&mut reply).await?;
    if reply[0] != 5 || reply[1] != 0 || reply[2] != 0 {
        return Err(io::Error::new(io::ErrorKind::Other, "SOCKS route unavailable"));
    }
    let address_len = match reply[3] {
        1 => 4,
        4 => 16,
        3 => stream.read_u8().await? as usize,
        _ => return Err(io::Error::new(io::ErrorKind::Other, "Invalid SOCKS address")),
    };
    let mut bound = vec![0; address_len + 2];
    stream.read_exact(&mut bound).await?;
    Ok(())
}

pub async fn run() {
    if option_env!("MOST_AUTO_PROXY") != Some("1") {
        return;
    }
    loop {
        if Config::get_option("most-auto-proxy") == "N" {
            return;
        }
        let current = Config::get_socks();
        if !managed(current.as_ref().map(|s| s.proxy.as_str()))
            || current.as_ref().map(|s| !s.username.is_empty() || !s.password.is_empty()).unwrap_or(false)
        {
            log::warn!("Most automatic routing stopped: custom proxy configured");
            return;
        }
        let available = matches!(timeout(Duration::from_secs(4), probe_proxy_at(PROXY)).await, Ok(Ok(())));
        // Recheck after the await so a user's newly configured proxy is never overwritten.
        let current = Config::get_socks();
        if !managed(current.as_ref().map(|s| s.proxy.as_str()))
            || current.as_ref().map(|s| !s.username.is_empty() || !s.password.is_empty()).unwrap_or(false)
        {
            return;
        }
        if available != current.is_some() {
            Config::set_socks(if available {
                Some(Socks5Server { proxy: PROXY.into(), username: String::new(), password: String::new() })
            } else {
                None
            });
            crate::RendezvousMediator::restart();
            crate::test_nat_type();
            log::info!("Most route changed: {}", if available { "HAPP SOCKS5" } else { "direct" });
        }
        sleep(Duration::from_secs(10)).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn most_auto_proxy_preserves_custom_proxies() {
        assert!(managed(None));
        assert!(managed(Some(PROXY)));
        assert!(!managed(Some("127.0.0.1:9050")));
        assert!(!managed(Some("proxy.example:1080")));
    }
    #[tokio::test]
    async fn most_auto_proxy_rejects_listener_without_socks() -> io::Result<()> {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
        let addr = listener.local_addr()?.to_string();
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await?;
            let mut greeting = [0; 3];
            socket.read_exact(&mut greeting).await?;
            socket.write_all(&[5, 255]).await
        });
        assert!(probe_proxy_at(&addr).await.is_err());
        server.await.map_err(|e| io::Error::new(io::ErrorKind::Other, e))??;
        Ok(())
    }

}
