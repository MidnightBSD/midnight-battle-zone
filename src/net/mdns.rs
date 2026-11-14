use std::net::{SocketAddr, UdpSocket};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanLobbyInfo {
    pub host_name: String,
    pub map_id: String,
    pub host_addr: SocketAddr,
    pub last_seen_ms: u64,
}

pub struct MdnsService {
    is_host: bool,
    host_name: String,
    map_id: String,
    game_port: u16,
    socket: UdpSocket,
    broadcast_socket: Option<UdpSocket>,
    last_announce_ms: u64,
}

impl MdnsService {
    pub fn new(is_host: bool, host_name: String, map_id: String, game_port: u16) -> std::io::Result<Self> {
        let socket = UdpSocket::bind("0.0.0.0:5353")
            .or_else(|_| UdpSocket::bind("0.0.0.0:0"))?;
        socket.set_nonblocking(true)?;

        let _ = socket.join_multicast_v4(&"224.0.0.251".parse().unwrap(), &std::net::Ipv4Addr::UNSPECIFIED);

        let broadcast_socket = if is_host {
            let bsock = UdpSocket::bind("0.0.0.0:45353").ok();
            if let Some(ref s) = bsock {
                let _ = s.set_broadcast(true);
                let _ = s.set_nonblocking(true);
            }
            bsock
        } else {
            let bsock = UdpSocket::bind("0.0.0.0:0").ok();
            if let Some(ref s) = bsock {
                let _ = s.set_broadcast(true);
                let _ = s.set_nonblocking(true);
            }
            bsock
        };

        Ok(Self {
            is_host,
            host_name,
            map_id,
            game_port,
            socket,
            broadcast_socket,
            last_announce_ms: 0,
        })
    }

    pub fn poll(&mut self, now_ms: u64) -> Vec<LanLobbyInfo> {
        let mut lobbies = Vec::new();

        if self.is_host {
            if now_ms.saturating_sub(self.last_announce_ms) >= 1000 {
                self.last_announce_ms = now_ms;
                let packet = encode_mdns_packet(&self.host_name, &self.map_id, self.game_port);
                let _ = self.socket.send_to(&packet, "224.0.0.251:5353");
                if let Some(ref bsock) = self.broadcast_socket {
                    let beacon = format!("mbz1|{}|{}|{}", self.host_name, self.map_id, self.game_port);
                    let _ = bsock.send_to(beacon.as_bytes(), "255.255.255.255:45353");
                    let _ = bsock.send_to(&packet, "224.0.0.251:45353");
                }
            }
        } else {
            if now_ms.saturating_sub(self.last_announce_ms) >= 3000 {
                self.last_announce_ms = now_ms;
                let query = encode_mdns_query();
                let _ = self.socket.send_to(&query, "224.0.0.251:5353");
                if let Some(ref bsock) = self.broadcast_socket {
                    let _ = bsock.send_to(b"MBZ_DISCOVER", "255.255.255.255:45353");
                }
            }
        }

        let mut buf = [0u8; 1500];
        loop {
            match self.socket.recv_from(&mut buf) {
                Ok((len, src)) => {
                    if let Some(lobby) = decode_mdns_packet(&buf[..len], src, now_ms) {
                        lobbies.push(lobby);
                    }
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                Err(_) => break,
            }
        }

        if let Some(ref bsock) = self.broadcast_socket {
            loop {
                match bsock.recv_from(&mut buf) {
                    Ok((len, src)) => {
                        let text = String::from_utf8_lossy(&buf[..len]);
                        if text.starts_with("mbz1|") {
                            let parts: Vec<&str> = text.split('|').collect();
                            if parts.len() >= 4 {
                                if let Ok(port) = parts[3].parse::<u16>() {
                                    let mut host_addr = src;
                                    host_addr.set_port(port);
                                    lobbies.push(LanLobbyInfo {
                                        host_name: parts[1].to_string(),
                                        map_id: parts[2].to_string(),
                                        host_addr,
                                        last_seen_ms: now_ms,
                                    });
                                }
                            }
                        } else if let Some(lobby) = decode_mdns_packet(&buf[..len], src, now_ms) {
                            lobbies.push(lobby);
                        }
                    }
                    Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                    Err(_) => break,
                }
            }
        }

        lobbies
    }
}

pub fn encode_mdns_query() -> Vec<u8> {
    let mut pkt = Vec::new();
    pkt.extend_from_slice(&[0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);
    for part in ["_mbz-rts", "_udp", "local"] {
        pkt.push(part.len() as u8);
        pkt.extend_from_slice(part.as_bytes());
    }
    pkt.push(0);
    pkt.extend_from_slice(&[0x00, 0x0c]);
    pkt.extend_from_slice(&[0x00, 0x01]);
    pkt
}

pub fn encode_mdns_packet(host_name: &str, map_id: &str, game_port: u16) -> Vec<u8> {
    let mut pkt = Vec::new();
    pkt.extend_from_slice(&[0x00, 0x00, 0x84, 0x00, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0x00]);

    let service_qname: &[&[u8]] = &[b"_mbz-rts", b"_udp", b"local"];
    let txt_data = format!("mbz1|{}|{}|{}", host_name, map_id, game_port);

    // 1. TXT Record
    for part in service_qname {
        pkt.push(part.len() as u8);
        pkt.extend_from_slice(*part);
    }
    pkt.push(0);
    pkt.extend_from_slice(&[0x00, 0x10]);
    pkt.extend_from_slice(&[0x80, 0x01]);
    pkt.extend_from_slice(&[0x00, 0x00, 0x00, 0x78]);
    let txt_len = txt_data.len() + 1;
    pkt.extend_from_slice(&(txt_len as u16).to_be_bytes());
    pkt.push(txt_data.len() as u8);
    pkt.extend_from_slice(txt_data.as_bytes());

    // 2. SRV Record
    for part in service_qname {
        pkt.push(part.len() as u8);
        pkt.extend_from_slice(*part);
    }
    pkt.push(0);
    pkt.extend_from_slice(&[0x00, 0x21]);
    pkt.extend_from_slice(&[0x80, 0x01]);
    pkt.extend_from_slice(&[0x00, 0x00, 0x00, 0x78]);
    let target_name = format!("{}.local.", host_name);
    let srv_rdata_len = 6 + target_name.len();
    pkt.extend_from_slice(&(srv_rdata_len as u16).to_be_bytes());
    pkt.extend_from_slice(&[0x00, 0x00]);
    pkt.extend_from_slice(&[0x00, 0x00]);
    pkt.extend_from_slice(&game_port.to_be_bytes());
    for part in target_name.split('.') {
        if !part.is_empty() {
            pkt.push(part.len() as u8);
            pkt.extend_from_slice(part.as_bytes());
        }
    }
    pkt.push(0);

    // 3. A Record
    for part in target_name.split('.') {
        if !part.is_empty() {
            pkt.push(part.len() as u8);
            pkt.extend_from_slice(part.as_bytes());
        }
    }
    pkt.push(0);
    pkt.extend_from_slice(&[0x00, 0x01]);
    pkt.extend_from_slice(&[0x80, 0x01]);
    pkt.extend_from_slice(&[0x00, 0x00, 0x00, 0x78]);
    pkt.extend_from_slice(&[0x00, 0x04]);
    pkt.extend_from_slice(&[127, 0, 0, 1]);

    pkt
}

pub fn decode_mdns_packet(buf: &[u8], src: SocketAddr, now_ms: u64) -> Option<LanLobbyInfo> {
    if buf.len() < 12 {
        return None;
    }
    let text = String::from_utf8_lossy(buf);
    if text.starts_with("mbz1|") {
        let parts: Vec<&str> = text.split('|').collect();
        if parts.len() >= 4 {
            if let Ok(port) = parts[3].parse::<u16>() {
                let mut host_addr = src;
                host_addr.set_port(port);
                return Some(LanLobbyInfo {
                    host_name: parts[1].to_string(),
                    map_id: parts[2].to_string(),
                    host_addr,
                    last_seen_ms: now_ms,
                });
            }
        }
    }

    if let Some(idx) = text.find("mbz1|") {
        let sub = &text[idx..];
        let parts: Vec<&str> = sub.split('|').collect();
        if parts.len() >= 4 {
            let port_str: String = parts[3].chars().take_while(|c| c.is_ascii_digit()).collect();
            if let Ok(port) = port_str.parse::<u16>() {
                let mut host_addr = src;
                host_addr.set_port(port);
                return Some(LanLobbyInfo {
                    host_name: parts[1].to_string(),
                    map_id: parts[2].to_string(),
                    host_addr,
                    last_seen_ms: now_ms,
                });
            }
        }
    }

    None
}
