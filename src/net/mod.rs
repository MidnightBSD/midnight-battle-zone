pub mod mdns;
pub mod lockstep;

pub use mdns::{MdnsService, LanLobbyInfo, encode_mdns_packet, decode_mdns_packet};
pub use lockstep::{LockstepSession, NetPacket, ReadyTick};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::GameCommand;

    #[test]
    fn test_mdns_roundtrip() {
        let host_name = "TestHost";
        let map_id = "test_map_01";
        let port = 7777;

        let packet = encode_mdns_packet(host_name, map_id, port);
        let src = "127.0.0.1:7777".parse().unwrap();
        let lobby = decode_mdns_packet(&packet, src, 1000).expect("Should decode successfully");

        assert_eq!(lobby.host_name, host_name);
        assert_eq!(lobby.map_id, map_id);
        assert_eq!(lobby.host_addr.port(), port);
    }

    #[test]
    fn test_lockstep_session_exchange() {
        let mut host = LockstepSession::host(0, "map_valkyrie".to_string()).expect("Host bind failed");
        let host_port = host.local_addr().unwrap().port();
        let server_addr = format!("127.0.0.1:{}", host_port).parse().unwrap();

        let mut client = LockstepSession::connect(server_addr, "ClientPlayer".to_string()).expect("Client connect failed");

        for _ in 0..10 {
            host.poll_network();
            client.poll_network();
            std::thread::sleep(std::time::Duration::from_millis(5));
        }

        assert!(host.is_connected(), "Host should be connected");
        assert!(client.is_connected(), "Client should be connected");

        for tick in 1..=10 {
            let host_cmd = GameCommand::MoveUnits {
                team: 0,
                unit_ids: vec![1, 2],
                target: [10.0, 20.0],
                attack_move: false,
            };
            let client_cmd = GameCommand::TrainUnit {
                team: 1,
                building_id: 100,
                unit_type: "tank".to_string(),
            };

            host.submit_local_tick(tick, vec![host_cmd.clone()], 0xABCDEF01);
            client.submit_local_tick(tick, vec![client_cmd.clone()], 0xABCDEF01);

            let mut ready_host = None;
            let mut ready_client = None;

            for _ in 0..50 {
                host.poll_network();
                client.poll_network();

                if let Some(r) = host.take_ready_tick(tick) {
                    ready_host = Some(r);
                }
                if let Some(r) = client.take_ready_tick(tick) {
                    ready_client = Some(r);
                }

                if ready_host.is_some() && ready_client.is_some() {
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(2));
            }

            let (host_cmds, host_chk) = match ready_host.expect("Host should have ready tick") {
                ReadyTick::Commands { commands, checksum } => (commands, checksum),
                ReadyTick::Desync { .. } => panic!("matching checksums must not desync"),
            };
            let (client_cmds, client_chk) = match ready_client.expect("Client should have ready tick") {
                ReadyTick::Commands { commands, checksum } => (commands, checksum),
                ReadyTick::Desync { .. } => panic!("matching checksums must not desync"),
            };

            assert_eq!(host_cmds.len(), 2);
            assert_eq!(client_cmds.len(), 2);
            assert_eq!(host_cmds, client_cmds);
            assert_eq!(host_chk, Some(0xABCDEF01));
            assert_eq!(client_chk, Some(0xABCDEF01));
        }
    }

    #[test]
    fn test_lockstep_checksum_mismatch_is_reported() {
        let mut host = LockstepSession::host(0, "map_valkyrie".to_string()).expect("Host bind failed");
        let server_addr = format!("127.0.0.1:{}", host.local_addr().unwrap().port())
            .parse()
            .unwrap();
        let mut client = LockstepSession::connect(server_addr, "ClientPlayer".to_string())
            .expect("Client connect failed");

        for _ in 0..10 {
            host.poll_network();
            client.poll_network();
            std::thread::sleep(std::time::Duration::from_millis(5));
        }

        host.submit_local_tick(1, Vec::new(), 0x1111_1111);
        client.submit_local_tick(1, Vec::new(), 0x2222_2222);

        let mut host_result = None;
        let mut client_result = None;
        for _ in 0..50 {
            host.poll_network();
            client.poll_network();
            host_result = host_result.or_else(|| host.take_ready_tick(1));
            client_result = client_result.or_else(|| client.take_ready_tick(1));
            if host_result.is_some() && client_result.is_some() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(2));
        }

        for result in [host_result, client_result] {
            assert!(matches!(
                result,
                Some(ReadyTick::Desync {
                    tick: 1,
                    local_checksum: _,
                    remote_checksum: _,
                })
            ));
        }
    }

    #[test]
    fn test_lockstep_rejects_duplicate_local_tick_submission() {
        let mut host = LockstepSession::host(0, "map_valkyrie".to_string()).expect("Host bind failed");
        let server_addr = format!("127.0.0.1:{}", host.local_addr().unwrap().port())
            .parse()
            .unwrap();
        let mut client = LockstepSession::connect(server_addr, "ClientPlayer".to_string())
            .expect("Client connect failed");

        for _ in 0..10 {
            host.poll_network();
            client.poll_network();
            std::thread::sleep(std::time::Duration::from_millis(5));
        }

        let first_command = GameCommand::MoveUnits {
            team: 0,
            unit_ids: vec![1],
            target: [10.0, 20.0],
            attack_move: false,
        };
        host.submit_local_tick(1, vec![first_command.clone()], 0xABCD_EF01);
        host.submit_local_tick(1, Vec::new(), 0xABCD_EF01);
        client.submit_local_tick(1, Vec::new(), 0xABCD_EF01);

        let mut client_result = None;
        for _ in 0..50 {
            host.poll_network();
            client.poll_network();
            let _ = host.take_ready_tick(1);
            client_result = client_result.or_else(|| client.take_ready_tick(1));
            if client_result.is_some() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(2));
        }

        match client_result.expect("Client should have ready tick") {
            ReadyTick::Commands { commands, .. } => assert_eq!(commands, vec![first_command]),
            ReadyTick::Desync { .. } => panic!("matching checksums must not desync"),
        }
    }

    #[test]
    fn test_lockstep_rejects_untrusted_and_wrong_team_packets() {
        let mut host = LockstepSession::host(0, "map_valkyrie".to_string()).expect("Host bind failed");
        let host_addr = host.local_addr().unwrap();
        let mut client = LockstepSession::connect(host_addr, "ClientPlayer".to_string())
            .expect("Client connect failed");

        for _ in 0..10 {
            host.poll_network();
            client.poll_network();
            std::thread::sleep(std::time::Duration::from_millis(5));
        }

        let attacker = std::net::UdpSocket::bind("127.0.0.1:0").expect("Attacker bind failed");
        let forged_pause = serde_json::to_vec(&NetPacket::PauseState { paused: true }).unwrap();
        attacker.send_to(&forged_pause, host_addr).unwrap();
        host.poll_network();
        assert_eq!(host.take_pause_update(), None);

        host.submit_local_tick(1, Vec::new(), 0xABCD_EF01);
        let forged_frame = NetPacket::TurnFrame {
            tick: 1,
            team: 0,
            commands: vec![GameCommand::StopUnits {
                team: 0,
                unit_ids: vec![1],
            }],
            state_checksum: 0xABCD_EF01,
        };
        client.send_test_packet(&forged_frame, host_addr).unwrap();
        host.poll_network();
        assert!(host.take_ready_tick(1).is_none());
    }
}
