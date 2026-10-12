//! libp2p networking: TCP + Noise (encryption) + Yamux, gossipsub for
//! listings/ratings/receipts/reports, Kademlia for peer discovery.
//! Received envelopes are handed to the caller unverified; callers must
//! verify (the ledger does) before storing or re-publishing.

use futures::StreamExt;
use libp2p::{gossipsub, identify, identity, kad, noise, swarm::NetworkBehaviour, swarm::SwarmEvent, tcp, yamux, Multiaddr, PeerId, Swarm};
use lc_core::Envelope;
use std::time::Duration;
use tokio::sync::mpsc;

pub const TOPIC: &str = "lastchange/v1/envelopes";

#[derive(NetworkBehaviour)]
pub struct Behaviour {
    pub gossipsub: gossipsub::Behaviour,
    pub kad: kad::Behaviour<kad::store::MemoryStore>,
    pub identify: identify::Behaviour,
}

pub enum Command { Publish(Envelope), Dial(Multiaddr) }

#[derive(Debug)]
pub enum Event { Listening(Multiaddr), Connected(PeerId), Received(Envelope) }

/// Build a swarm. The network key is a fresh ephemeral key, NOT the user's
/// marketplace identity, so transport peer ids are unlinkable to pseudonyms.
pub fn build_swarm() -> anyhow::Result<Swarm<Behaviour>> {
    let swarm = libp2p::SwarmBuilder::with_new_identity()
        .with_tokio()
        .with_tcp(tcp::Config::default(), noise::Config::new, yamux::Config::default)?
        .with_behaviour(|key: &identity::Keypair| {
            let cfg = gossipsub::ConfigBuilder::default()
                .heartbeat_interval(Duration::from_millis(500))
                .validation_mode(gossipsub::ValidationMode::Strict)
                .max_transmit_size(64 * 1024)
                .build()
                .map_err(|e| anyhow::anyhow!(e))?;
            let gossipsub = gossipsub::Behaviour::new(gossipsub::MessageAuthenticity::Signed(key.clone()), cfg)
                .map_err(|e| anyhow::anyhow!(e))?;
            let pid = key.public().to_peer_id();
            let mut kad = kad::Behaviour::new(pid, kad::store::MemoryStore::new(pid));
            kad.set_mode(Some(kad::Mode::Server));
            let identify = identify::Behaviour::new(identify::Config::new("/lastchange/1.0".into(), key.public()));
            Ok(Behaviour { gossipsub, kad, identify })
        })?
        .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(60)))
        .build();
    Ok(swarm)
}

/// Run the node loop: listen on `listen`, handle commands, emit events.
pub async fn run(listen: Multiaddr, mut cmds: mpsc::Receiver<Command>, events: mpsc::Sender<Event>) -> anyhow::Result<()> {
    let mut swarm = build_swarm()?;
    let topic = gossipsub::IdentTopic::new(TOPIC);
    swarm.behaviour_mut().gossipsub.subscribe(&topic)?;
    swarm.listen_on(listen)?;
    loop {
        tokio::select! {
            cmd = cmds.recv() => match cmd {
                None => return Ok(()),
                Some(Command::Dial(a)) => { swarm.dial(a)?; }
                Some(Command::Publish(env)) => {
                    let _ = swarm.behaviour_mut().gossipsub.publish(topic.clone(), serde_json::to_vec(&env)?);
                }
            },
            ev = swarm.select_next_some() => match ev {
                SwarmEvent::NewListenAddr { address, .. } => { let _ = events.send(Event::Listening(address)).await; }
                SwarmEvent::ConnectionEstablished { peer_id, endpoint, .. } => {
                    swarm.behaviour_mut().kad.add_address(&peer_id, endpoint.get_remote_address().clone());
                    swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer_id);
                    let _ = events.send(Event::Connected(peer_id)).await;
                }
                SwarmEvent::Behaviour(BehaviourEvent::Identify(identify::Event::Received { peer_id, info, .. })) => {
                    for a in info.listen_addrs { swarm.behaviour_mut().kad.add_address(&peer_id, a); }
                }
                SwarmEvent::Behaviour(BehaviourEvent::Gossipsub(gossipsub::Event::Message { message, .. })) => {
                    if let Ok(env) = serde_json::from_slice::<Envelope>(&message.data) {
                        let _ = events.send(Event::Received(env)).await;
                    }
                }
                _ => {}
            }
        }
    }
}
