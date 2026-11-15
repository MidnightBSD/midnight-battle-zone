use std::collections::{HashMap, VecDeque};
use std::net::{SocketAddr, UdpSocket};
use serde::{Deserialize, Serialize};
use crate::types::{TeamId, GameCommand};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NetPacket {
    Hello { player_name: String },
    Welcome { assigned_team: TeamId, map_id: String, seed: u64 },
    TurnFrame { tick: u64, team: TeamId, commands: Vec<GameCommand>, state_checksum: u32 },
    Ack { tick: u64, team: TeamId },
    PauseState { paused: bool },
    Ping { timestamp: u64 },
    Pong { timestamp: u64 },
}

#[derive(Debug, Clone, PartialEq)]
pub enum ReadyTick {
    Commands {
        commands: Vec<GameCommand>,
        checksum: Option<u32>,
    },
    Desync {
        tick: u64,
        local_checksum: u32,
        remote_checksum: u32,
    },
}

#[derive(Debug, Clone)]
struct PendingFrame {
    packet: NetPacket,
    #[allow(dead_code)]
    last_sent_ms: u64,
}

pub struct LockstepSession {
    socket: UdpSocket,
    remote_addr: Option<SocketAddr>,
    is_host: bool,
    local_team: TeamId,
    connected: bool,
    map_id: String,
    seed: u64,
    pending_pause_update: Option<bool>,
    
    outbox_frames: VecDeque<PendingFrame>,
    remote_frames: HashMap<u64, HashMap<TeamId, (Vec<GameCommand>, Option<u32>)>>,
    local_frames: HashMap<u64, (Vec<GameCommand>, Option<u32>)>,
    remote_acked_tick: u64,
    last_tick_sent: u64,
    #[allow(dead_code)]
    last_poll_ms: u64,
}

impl LockstepSession {
    fn remote_team(&self) -> TeamId {
        if self.local_team == 0 { 1 } else { 0 }
    }

    fn is_trusted_peer(&self, src: SocketAddr) -> bool {
        self.remote_addr == Some(src)
    }

    pub fn host(bind_port: u16, map_id: String) -> std::io::Result<Self> {
        let socket = UdpSocket::bind(format!("0.0.0.0:{}", bind_port))?;
        socket.set_nonblocking(true)?;
        let seed = rand::random();
        Ok(Self {
            socket,
            remote_addr: None,
            is_host: true,
            local_team: 0,
            connected: false,
            map_id,
            seed,
            pending_pause_update: None,
            outbox_frames: VecDeque::new(),
            remote_frames: HashMap::new(),
            local_frames: HashMap::new(),
            remote_acked_tick: 0,
            last_tick_sent: 0,
            last_poll_ms: 0,
        })
    }

    pub fn connect(server_addr: SocketAddr, player_name: String) -> std::io::Result<Self> {
        let socket = UdpSocket::bind("0.0.0.0:0")?;
        socket.set_nonblocking(true)?;
        
        let session = Self {
            socket,
            remote_addr: Some(server_addr),
            is_host: false,
            local_team: 1,
            connected: false,
            map_id: String::new(),
            seed: 0,
            pending_pause_update: None,
            outbox_frames: VecDeque::new(),
            remote_frames: HashMap::new(),
            local_frames: HashMap::new(),
            remote_acked_tick: 0,
            last_tick_sent: 0,
            last_poll_ms: 0,
        };

        let pkt = NetPacket::Hello { player_name };
        if let Ok(bytes) = serde_json::to_vec(&pkt) {
            let _ = session.socket.send_to(&bytes, server_addr);
        }

        Ok(session)
    }

    pub fn local_team(&self) -> TeamId {
        self.local_team
    }

    pub fn is_connected(&self) -> bool {
        self.connected
    }

    pub fn local_addr(&self) -> std::io::Result<SocketAddr> {
        self.socket.local_addr()
    }

    #[cfg(test)]
    pub(crate) fn send_test_packet(
        &self,
        packet: &NetPacket,
        destination: SocketAddr,
    ) -> std::io::Result<usize> {
        let bytes = serde_json::to_vec(packet)
            .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?;
        self.socket.send_to(&bytes, destination)
    }

    pub fn map_id(&self) -> &str {
        &self.map_id
    }

    pub fn seed(&self) -> u64 {
        self.seed
    }

    pub fn send_pause_state(&mut self, paused: bool) {
        let pkt = NetPacket::PauseState { paused };
        if let Ok(bytes) = serde_json::to_vec(&pkt) {
            if let Some(addr) = self.remote_addr {
                let _ = self.socket.send_to(&bytes, addr);
            }
        }
    }

    pub fn take_pause_update(&mut self) -> Option<bool> {
        self.pending_pause_update.take()
    }

    pub fn has_submitted_tick(&self, tick: u64) -> bool {
        self.local_frames.contains_key(&tick)
    }

    pub fn submit_local_tick(&mut self, tick: u64, commands: Vec<GameCommand>, checksum: u32) {
        if self.has_submitted_tick(tick) {
            return;
        }

        self.local_frames.insert(tick, (commands.clone(), Some(checksum)));
        let pkt = NetPacket::TurnFrame {
            tick,
            team: self.local_team,
            commands,
            state_checksum: checksum,
        };
        if let Ok(bytes) = serde_json::to_vec(&pkt) {
            if let Some(addr) = self.remote_addr {
                let _ = self.socket.send_to(&bytes, addr);
            }
            self.outbox_frames.push_back(PendingFrame {
                packet: pkt,
                last_sent_ms: 0,
            });
        }
        self.last_tick_sent = tick;
    }

    pub fn poll_network(&mut self) {
        let mut buf = [0u8; 4096];
        loop {
            match self.socket.recv_from(&mut buf) {
                Ok((len, src)) => {
                    if let Ok(packet) = serde_json::from_slice(&buf[..len]) {
                        self.handle_packet(packet, src);
                    }
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                Err(_) => break,
            }
        }
    }

    fn handle_packet(&mut self, packet: NetPacket, src: SocketAddr) {
        match packet {
            NetPacket::Hello { .. }
                if self.is_host
                    && self.remote_addr.map_or(true, |remote_addr| remote_addr == src) =>
            {
                self.remote_addr = Some(src);
                self.connected = true;
                let welcome = NetPacket::Welcome {
                    assigned_team: 1,
                    map_id: self.map_id.clone(),
                    seed: self.seed,
                };
                if let Ok(bytes) = serde_json::to_vec(&welcome) {
                    let _ = self.socket.send_to(&bytes, src);
                }
            }
            NetPacket::Welcome { assigned_team, map_id, seed }
                if !self.is_host && self.is_trusted_peer(src) =>
            {
                self.local_team = assigned_team;
                self.map_id = map_id;
                self.seed = seed;
                self.connected = true;
            }
            NetPacket::TurnFrame { tick, team, commands, state_checksum }
                if self.is_trusted_peer(src)
                    && team == self.remote_team()
                    && commands.iter().all(|command| game_command_team(command) == team) =>
            {
                self.remote_frames
                    .entry(tick)
                    .or_default()
                    .insert(team, (commands, Some(state_checksum)));
                
                let ack = NetPacket::Ack { tick, team: self.local_team };
                if let Ok(bytes) = serde_json::to_vec(&ack) {
                    let _ = self.socket.send_to(&bytes, src);
                }
            }
            NetPacket::Ack { tick, .. } if self.is_trusted_peer(src) => {
                if tick > self.remote_acked_tick {
                    self.remote_acked_tick = tick;
                }
                self.outbox_frames.retain(|pf| {
                    if let NetPacket::TurnFrame { tick: t, .. } = pf.packet {
                        t > tick
                    } else {
                        true
                    }
                });
            }
            NetPacket::PauseState { paused } if self.is_trusted_peer(src) => {
                self.pending_pause_update = Some(paused);
            }
            NetPacket::Ping { timestamp } if self.is_trusted_peer(src) => {
                let pong = NetPacket::Pong { timestamp };
                if let Ok(bytes) = serde_json::to_vec(&pong) {
                    let _ = self.socket.send_to(&bytes, src);
                }
            }
            NetPacket::Pong { .. } if self.is_trusted_peer(src) => {}
            _ => {}
        }
    }

    pub fn take_ready_tick(&mut self, tick: u64) -> Option<ReadyTick> {
        let remote_team = self.remote_team();

        let (local_cmds, local_checksum) = self.local_frames.get(&tick)?.clone();
        let (remote_cmds, remote_checksum) = self
            .remote_frames
            .get(&tick)?
            .get(&remote_team)?
            .clone();

        self.local_frames.remove(&tick);
        let remove_remote_tick = if let Some(remotes) = self.remote_frames.get_mut(&tick) {
            remotes.remove(&remote_team);
            remotes.is_empty()
        } else {
            false
        };
        if remove_remote_tick {
            self.remote_frames.remove(&tick);
        }

        if let (Some(local), Some(remote)) = (local_checksum, remote_checksum) {
            if local != remote {
                return Some(ReadyTick::Desync {
                    tick,
                    local_checksum: local,
                    remote_checksum: remote,
                });
            }
        }

        let mut commands = Vec::with_capacity(local_cmds.len() + remote_cmds.len());
        let (team0_cmds, team1_cmds) = if self.local_team == 0 {
            (&local_cmds, &remote_cmds)
        } else {
            (&remote_cmds, &local_cmds)
        };
        commands.extend(team0_cmds.iter().cloned());
        commands.extend(team1_cmds.iter().cloned());

        Some(ReadyTick::Commands {
            commands,
            checksum: local_checksum.or(remote_checksum),
        })
    }
}

fn game_command_team(command: &GameCommand) -> TeamId {
    match command {
        GameCommand::MoveUnits { team, .. }
        | GameCommand::AttackTarget { team, .. }
        | GameCommand::ConstructBuilding { team, .. }
        | GameCommand::RepairBuilding { team, .. }
        | GameCommand::HarvestMine { team, .. }
        | GameCommand::TrainUnit { team, .. }
        | GameCommand::StopUnits { team, .. } => *team,
    }
}
