//! Embedded DNS server for hive proxy hostnames.
//!
//! Resolves configured hostnames (from `proxy.host` in hive.yaml) to 127.0.0.1
//! and forwards unknown queries to an upstream DNS server.

use crate::daemon_defaults;
use dashmap::DashMap;
use simple_dns::{Packet, PacketFlag, QTYPE, Question, ResourceRecord, CLASS, TYPE};
use std::collections::HashSet;
use std::net::{Ipv4Addr, SocketAddr};
use std::sync::Arc;
use tokio::net::UdpSocket;
use tokio::task::JoinHandle;
use tracing::{debug, error, info, warn};

#[derive(Debug, Clone)]
pub struct DnsConfig {
    pub enabled: bool,
    pub bind: String,
    pub upstream: String,
    pub ttl: u32,
}

impl Default for DnsConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            bind: daemon_defaults::DNS_BIND.to_string(),
            upstream: daemon_defaults::DNS_UPSTREAM.to_string(),
            ttl: daemon_defaults::DNS_TTL,
        }
    }
}

pub struct DnsServer {
    records: Arc<DashMap<String, Ipv4Addr>>,
    config: DnsConfig,
}

impl DnsServer {
    pub fn new(config: DnsConfig) -> Self {
        Self {
            records: Arc::new(DashMap::new()),
            config,
        }
    }

    pub fn sync_records(&self, hosts: &[String], ip: Ipv4Addr) {
        self.records.clear();
        for host in hosts {
            self.records.insert(normalize_hostname(host), ip);
        }
        debug!("DNS records synced: {} hostnames -> {}", hosts.len(), ip);
    }

    pub fn add_record(&self, hostname: &str, ip: Ipv4Addr) {
        self.records.insert(normalize_hostname(hostname), ip);
    }

    pub fn remove_record(&self, hostname: &str) {
        self.records.remove(&normalize_hostname(hostname));
    }

    fn lookup(&self, hostname: &str) -> Option<Ipv4Addr> {
        self.records.get(&normalize_hostname(hostname)).map(|r| *r)
    }

    pub fn hostnames(&self) -> Vec<String> {
        self.records.iter().map(|r| r.key().clone()).collect()
    }
}

fn normalize_hostname(h: &str) -> String {
    h.to_lowercase().trim_end_matches('.').to_string()
}

pub fn start_dns_server(server: Arc<DnsServer>) -> anyhow::Result<JoinHandle<()>> {
    let bind_addr: SocketAddr = server.config.bind.parse().map_err(|e| {
        anyhow::anyhow!("Invalid DNS bind address '{}': {}", server.config.bind, e)
    })?;
    let upstream: SocketAddr = server.config.upstream.parse().map_err(|e| {
        anyhow::anyhow!(
            "Invalid DNS upstream address '{}': {}",
            server.config.upstream,
            e
        )
    })?;

    let std_socket = std::net::UdpSocket::bind(bind_addr)
        .map_err(|e| anyhow::anyhow!("Failed to bind DNS on {}: {}", bind_addr, e))?;
    std_socket.set_nonblocking(true)?;

    info!("DNS server started on {}", bind_addr);

    let handle = tokio::spawn(async move {
        let socket = match UdpSocket::from_std(std_socket) {
            Ok(s) => Arc::new(s),
            Err(e) => {
                error!("Failed to convert DNS socket: {}", e);
                return;
            }
        };

        let mut buf = vec![0u8; 512];
        loop {
            let (len, src) = match socket.recv_from(&mut buf).await {
                Ok(r) => r,
                Err(e) => {
                    error!("DNS recv error: {}", e);
                    continue;
                }
            };

            let data = buf[..len].to_vec();
            let server = server.clone();
            let socket = socket.clone();

            tokio::spawn(async move {
                let response = handle_query(&data, &server, upstream).await;
                match response {
                    Ok(resp) => {
                        if let Err(e) = socket.send_to(&resp, src).await {
                            debug!("DNS send error to {}: {}", src, e);
                        }
                    }
                    Err(e) => {
                        debug!("DNS query handling error: {}", e);
                    }
                }
            });
        }
    });

    Ok(handle)
}

async fn handle_query(
    data: &[u8],
    server: &DnsServer,
    upstream: SocketAddr,
) -> anyhow::Result<Vec<u8>> {
    let packet = Packet::parse(data)?;
    let question = packet
        .questions
        .first()
        .ok_or_else(|| anyhow::anyhow!("No question in DNS packet"))?;

    let qname = question.qname.to_string();
    let normalized = normalize_hostname(&qname);

    if question.qtype == QTYPE::TYPE(TYPE::A) {
        if let Some(ip) = server.lookup(&normalized) {
            debug!("DNS resolved {} -> {}", normalized, ip);
            return build_a_response(&packet, question, ip, server.config.ttl);
        }
    }

    forward_to_upstream(data, upstream).await
}

fn build_a_response(
    query: &Packet,
    question: &Question,
    ip: Ipv4Addr,
    ttl: u32,
) -> anyhow::Result<Vec<u8>> {
    let mut response = Packet::new_reply(query.id());
    response.set_flags(PacketFlag::RESPONSE | PacketFlag::RECURSION_AVAILABLE);

    response.questions.push(Question::new(
        question.qname.clone(),
        question.qtype,
        question.qclass,
        question.unicast_response,
    ));

    let rdata = simple_dns::rdata::RData::A(simple_dns::rdata::A { address: ip.into() });
    response.answers.push(ResourceRecord::new(
        question.qname.clone(),
        CLASS::IN,
        ttl,
        rdata,
    ));

    Ok(response.build_bytes_vec()?)
}

async fn forward_to_upstream(data: &[u8], upstream: SocketAddr) -> anyhow::Result<Vec<u8>> {
    let socket = UdpSocket::bind("0.0.0.0:0").await?;
    socket.send_to(data, upstream).await?;

    let mut buf = vec![0u8; 512];
    let len = tokio::time::timeout(std::time::Duration::from_secs(2), socket.recv(&mut buf))
        .await
        .map_err(|_| anyhow::anyhow!("Upstream DNS timeout"))??;

    Ok(buf[..len].to_vec())
}

pub fn collect_tlds(hosts: &[String]) -> HashSet<String> {
    hosts
        .iter()
        .filter_map(|h| {
            let h = h.trim_end_matches('.');
            h.rsplit('.').next().map(|tld| tld.to_lowercase())
        })
        .collect()
}

#[cfg(target_os = "macos")]
fn privileged_user() -> Option<String> {
    std::env::var("ADI_ROOT_USER").ok().filter(|s| !s.is_empty())
}

/// Run a command via the adi-root privilege chain.
///
/// Executes `sudo -u adi-root sudo <program> <args>` — the first sudo
/// switches to adi-root (NOPASSWD), then adi-root runs sudo (also NOPASSWD).
/// Returns None if ADI_ROOT_USER is not configured.
#[cfg(target_os = "macos")]
fn privileged_command(program: &str, args: &[&str]) -> Option<std::process::Command> {
    let root_user = privileged_user()?;
    let mut cmd = std::process::Command::new("sudo");
    cmd.args(["-u", &root_user, "sudo", program]).args(args);
    Some(cmd)
}

#[cfg(target_os = "macos")]
pub fn ensure_resolver_files(tlds: &HashSet<String>, port: u16) -> anyhow::Result<()> {
    if tlds.is_empty() {
        return Ok(());
    }

    let Some(_) = privileged_user() else {
        debug!("Skipping resolver file creation: ADI_ROOT_USER not configured");
        return Ok(());
    };

    if !std::path::Path::new("/etc/resolver").exists() {
        let status = privileged_command("mkdir", &["-p", "/etc/resolver"])
            .expect("privileged_user is Some")
            .status();
        match status {
            Ok(s) if s.success() => info!("Created /etc/resolver directory"),
            Ok(s) => {
                warn!("Failed to create /etc/resolver directory (exit {})", s);
                return Ok(());
            }
            Err(e) => {
                warn!("Failed to create /etc/resolver directory: {}", e);
                return Ok(());
            }
        }
    }

    for tld in tlds {
        let path = format!("/etc/resolver/{}", tld);
        if std::path::Path::new(&path).exists() {
            debug!("Resolver file already exists: {}", path);
            continue;
        }

        let content = format!("nameserver 127.0.0.1\nport {}\n", port);

        let result = privileged_command("tee", &[&path])
            .expect("privileged_user is Some")
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::null())
            .spawn()
            .and_then(|mut child| {
                use std::io::Write;
                if let Some(stdin) = child.stdin.as_mut() {
                    stdin.write_all(content.as_bytes())?;
                }
                child.wait()
            })
            .and_then(|s| {
                if s.success() {
                    Ok(())
                } else {
                    Err(std::io::Error::other(format!("exit {}", s)))
                }
            });

        match result {
            Ok(()) => info!("Created resolver file: {}", path),
            Err(e) => warn!("Failed to create resolver file {}: {}", path, e),
        }
    }
    Ok(())
}

#[cfg(target_os = "macos")]
pub fn cleanup_resolver_files(tlds: &HashSet<String>) -> anyhow::Result<()> {
    if privileged_user().is_none() {
        debug!("Skipping resolver file cleanup: ADI_ROOT_USER not configured");
        return Ok(());
    }
    let mut removed = false;
    for tld in tlds {
        let path = format!("/etc/resolver/{}", tld);
        if !std::path::Path::new(&path).exists() {
            continue;
        }

        let result = privileged_command("rm", &["-f", &path])
            .expect("privileged_user is Some")
            .status()
            .and_then(|s| {
                if s.success() {
                    Ok(())
                } else {
                    Err(std::io::Error::other(format!("exit {}", s)))
                }
            });

        match result {
            Ok(()) => {
                info!("Removed resolver file: {}", path);
                removed = true;
            }
            Err(e) => warn!("Failed to remove resolver file {}: {}", path, e),
        }
    }
    if removed {
        flush_dns_cache();
    }
    Ok(())
}

/// Signal mDNSResponder to re-read `/etc/resolver/` files.
///
/// Without this, macOS may serve stale (negative) DNS cache entries
/// even after resolver files are created or removed.
#[cfg(target_os = "macos")]
pub fn flush_dns_cache() {
    let Some(mut cmd) = privileged_command("killall", &["-HUP", "mDNSResponder"]) else {
        debug!("Skipping DNS cache flush: ADI_ROOT_USER not configured");
        return;
    };
    match cmd.status() {
        Ok(s) if s.success() => debug!("Flushed macOS DNS cache (HUP mDNSResponder)"),
        Ok(s) => warn!("Failed to flush DNS cache (exit {})", s),
        Err(e) => warn!("Failed to flush DNS cache: {}", e),
    }
}

#[cfg(not(target_os = "macos"))]
pub fn flush_dns_cache() {}

#[cfg(not(target_os = "macos"))]
pub fn ensure_resolver_files(_tlds: &HashSet<String>, _port: u16) -> anyhow::Result<()> {
    Ok(())
}

#[cfg(not(target_os = "macos"))]
pub fn cleanup_resolver_files(_tlds: &HashSet<String>) -> anyhow::Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_hostname() {
        assert_eq!(normalize_hostname("MyApp.Test."), "myapp.test");
        assert_eq!(normalize_hostname("myapp.test"), "myapp.test");
    }

    #[test]
    fn test_collect_tlds() {
        let hosts = vec![
            "app.test".to_string(),
            "api.test".to_string(),
            "site.local".to_string(),
        ];
        let tlds = collect_tlds(&hosts);
        assert!(tlds.contains("test"));
        assert!(tlds.contains("local"));
        assert_eq!(tlds.len(), 2);
    }

    #[test]
    fn test_dns_server_records() {
        let server = DnsServer::new(DnsConfig::default());
        let ip = Ipv4Addr::new(127, 0, 0, 1);

        server.add_record("myapp.test", ip);
        assert_eq!(server.lookup("myapp.test"), Some(ip));
        assert_eq!(server.lookup("MYAPP.TEST"), Some(ip));

        server.remove_record("myapp.test");
        assert_eq!(server.lookup("myapp.test"), None);
    }

    #[test]
    fn test_sync_records() {
        let server = DnsServer::new(DnsConfig::default());
        let ip = Ipv4Addr::new(127, 0, 0, 1);

        server.add_record("old.test", ip);
        server.sync_records(&["new.test".to_string()], ip);

        assert_eq!(server.lookup("old.test"), None);
        assert_eq!(server.lookup("new.test"), Some(ip));
    }
}
