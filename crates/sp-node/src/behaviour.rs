use async_trait::async_trait;
use futures::prelude::*;
use libp2p::{
    gossipsub::{self, Behaviour as Gossipsub, MessageAuthenticity},
    identify::{self, Behaviour as Identify},
    kad::{store::MemoryStore, Behaviour as Kademlia},
    mdns::{self, tokio::Behaviour as Mdns},
    ping::{self, Behaviour as Ping},
    request_response::{self, Behaviour as RequestResponse, Codec, ProtocolSupport},
    swarm::NetworkBehaviour,
};

use crate::protocol::{TOPIC_BLOCK, TOPIC_TX, TOPIC_VERIFY};

/// Codec for the block sync request-response protocol.
///
/// Both request and response are raw byte vectors; serialisation/deserialisation
/// is handled in the node layer using `bincode`.
#[derive(Clone, Default)]
pub struct SyncCodec;

#[async_trait]
impl Codec for SyncCodec {
    type Protocol = String;
    type Request = Vec<u8>;
    type Response = Vec<u8>;

    async fn read_request<T>(
        &mut self,
        _protocol: &Self::Protocol,
        io: &mut T,
    ) -> std::io::Result<Self::Request>
    where
        T: AsyncRead + Unpin + Send,
    {
        let mut buf = Vec::new();
        io.read_to_end(&mut buf).await?;
        Ok(buf)
    }

    async fn read_response<T>(
        &mut self,
        _protocol: &Self::Protocol,
        io: &mut T,
    ) -> std::io::Result<Self::Response>
    where
        T: AsyncRead + Unpin + Send,
    {
        let mut buf = Vec::new();
        io.read_to_end(&mut buf).await?;
        Ok(buf)
    }

    async fn write_request<T>(
        &mut self,
        _protocol: &Self::Protocol,
        io: &mut T,
        req: Self::Request,
    ) -> std::io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        io.write_all(&req).await?;
        io.close().await
    }

    async fn write_response<T>(
        &mut self,
        _protocol: &Self::Protocol,
        io: &mut T,
        resp: Self::Response,
    ) -> std::io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        io.write_all(&resp).await?;
        io.close().await
    }
}

/// Combined libp2p behaviour that powers the Social Production P2P node.
#[derive(NetworkBehaviour)]
#[behaviour(prelude = "libp2p::swarm::derive_prelude")]
pub struct SpBehaviour {
    /// Epidemic broadcast — used for transactions, block announcements and
    /// block verifications.
    pub gossipsub: Gossipsub,

    /// Kademlia DHT — global peer discovery and routing.
    pub kademlia: Kademlia<MemoryStore>,

    /// mDNS — zero-config local network peer discovery.
    pub mdns: Mdns,

    /// Ping — periodic keepalive; detects and disconnects unresponsive peers.
    pub ping: Ping,

    /// Identify — exchange peer metadata on connection.
    pub identify: Identify,

    /// Request-response — direct block sync between two peers.
    pub request_response: RequestResponse<SyncCodec>,
}

/// Build the combined [`SpBehaviour`] for the given keypair.
pub fn build_behaviour(
    keypair: &libp2p::identity::Keypair,
) -> Result<SpBehaviour, Box<dyn std::error::Error + Send + Sync>> {
    let peer_id = keypair.public().to_peer_id();

    // Gossipsub
    let gossipsub_config = gossipsub::ConfigBuilder::default()
        .heartbeat_interval(std::time::Duration::from_secs(10))
        .validation_mode(gossipsub::ValidationMode::Strict)
        .build()
        .map_err(|e| format!("gossipsub config: {e}"))?;

    let mut gossipsub = Gossipsub::new(
        MessageAuthenticity::Signed(keypair.clone()),
        gossipsub_config,
    )
    .map_err(|e| format!("gossipsub init: {e}"))?;

    for topic_str in [TOPIC_TX, TOPIC_VERIFY, TOPIC_BLOCK] {
        let topic = gossipsub::IdentTopic::new(topic_str);
        gossipsub.subscribe(&topic)?;
    }

    // Kademlia
    let store = MemoryStore::new(peer_id);
    let kademlia = Kademlia::new(peer_id, store);

    // mDNS
    let mdns = Mdns::new(mdns::Config::default(), peer_id)?;

    // Ping — pings each connected peer every 15 s; disconnects after 3 timeouts.
    let ping = Ping::new(ping::Config::new());

    // Identify
    let identify = Identify::new(identify::Config::new(
        "/sp/1.0.0".into(),
        keypair.public(),
    ));

    // Request-response (block sync)
    let request_response = RequestResponse::new(
        [(
            "/sp/sync/1.0.0".to_string(),
            ProtocolSupport::Full,
        )],
        request_response::Config::default(),
    );

    Ok(SpBehaviour {
        gossipsub,
        kademlia,
        mdns,
        ping,
        identify,
        request_response,
    })
}
