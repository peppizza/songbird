use super::message::*;
use crate::constants::*;
use discortp::discord::MutableKeepalivePacket;
use flume::Receiver;
use std::sync::Arc;
use tokio::{
    net::UdpSocket,
    time::{timeout_at, Instant},
};
use tracing::{error, info, instrument, trace};

#[instrument(skip(udp_msg_rx))]
pub(crate) async fn runner(udp_msg_rx: Receiver<UdpTxMessage>, ssrc: u32, udp_tx: Arc<UdpSocket>) {
    info!("UDP transmit handle started.");

    let mut keepalive_bytes = [0u8; MutableKeepalivePacket::minimum_packet_size()];
    let mut ka = MutableKeepalivePacket::new(&mut keepalive_bytes[..])
        .expect("FATAL: Insufficient bytes given to keepalive packet.");
    ka.set_ssrc(ssrc);

    let mut ka_time = Instant::now() + UDP_KEEPALIVE_GAP;

    loop {
        use UdpTxMessage::*;
        match timeout_at(ka_time, udp_msg_rx.recv_async()).await {
            Err(_) => {
                trace!("Sending UDP Keepalive.");
                if let Err(e) = udp_tx.send(&keepalive_bytes[..]).await {
                    error!("Fatal UDP keepalive send error: {:?}.", e);
                    break;
                }
                ka_time += UDP_KEEPALIVE_GAP;
            },
            Ok(Ok(Packet(p))) =>
                if let Err(e) = udp_tx.send(&p[..]).await {
                    error!("Fatal UDP packet send error: {:?}.", e);
                    break;
                },
            Ok(Err(e)) => {
                error!("Fatal UDP packet receive error: {:?}.", e);
                break;
            },
            Ok(Ok(Poison)) => {
                break;
            },
        }
    }

    info!("UDP transmit handle stopped.");
}
