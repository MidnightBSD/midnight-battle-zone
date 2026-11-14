pub mod mdns;
pub mod lockstep;

pub use mdns::{MdnsService, LanLobbyInfo, encode_mdns_packet, decode_mdns_packet};
pub use lockstep::{LockstepSession, NetPacket};

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

            let (host_cmds, host_chk) = ready_host.expect("Host should have ready tick");
            let (client_cmds, client_chk) = ready_client.expect("Client should have ready tick");

            assert_eq!(host_cmds.len(), 2);
            assert_eq!(client_cmds.len(), 2);
            assert_eq!(host_cmds, client_cmds);
            assert_eq!(host_chk, Some(0xABCDEF01));
            assert_eq!(client_chk, Some(0xABCDEF01));
        }
    }
}
