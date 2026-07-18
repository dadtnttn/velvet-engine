//! Minimal netcode peer surface with deterministic loopback round-trip.
//!
//! Not a full multiplayer stack — provides encode/decode + loopback transport
//! so hosts can ship a real integration point and tests prove message delivery.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Netcode errors.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum NetError {
    /// Empty payload.
    #[error("empty payload")]
    Empty,
    /// Decode failed.
    #[error("decode: {0}")]
    Decode(String),
    /// Peer closed.
    #[error("peer closed")]
    Closed,
}

/// Application-level message.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetMessage {
    /// Channel / kind id.
    pub channel: u16,
    /// Sequence number.
    pub seq: u32,
    /// UTF-8 payload (story choice, save slot, etc.).
    pub body: String,
}

impl NetMessage {
    /// Create a message.
    pub fn new(channel: u16, seq: u32, body: impl Into<String>) -> Self {
        Self {
            channel,
            seq,
            body: body.into(),
        }
    }

    /// Encode to a length-prefixed wire frame: `ch:u16|seq:u32|body`.
    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(&self.channel.to_le_bytes());
        out.extend_from_slice(&self.seq.to_le_bytes());
        let body = self.body.as_bytes();
        let len = body.len() as u32;
        out.extend_from_slice(&len.to_le_bytes());
        out.extend_from_slice(body);
        out
    }

    /// Decode from wire bytes.
    pub fn decode(bytes: &[u8]) -> Result<Self, NetError> {
        if bytes.len() < 10 {
            return Err(NetError::Decode("frame too short".into()));
        }
        let channel = u16::from_le_bytes([bytes[0], bytes[1]]);
        let seq = u32::from_le_bytes([bytes[2], bytes[3], bytes[4], bytes[5]]);
        let len = u32::from_le_bytes([bytes[6], bytes[7], bytes[8], bytes[9]]) as usize;
        if bytes.len() < 10 + len {
            return Err(NetError::Decode("truncated body".into()));
        }
        let body = std::str::from_utf8(&bytes[10..10 + len])
            .map_err(|e| NetError::Decode(e.to_string()))?
            .to_string();
        if body.is_empty() && len == 0 {
            // allow empty body
        }
        Ok(Self { channel, seq, body })
    }
}

/// Simple peer with an inbox/outbox (loopback wiring).
#[derive(Debug, Default)]
pub struct NetPeer {
    /// Local peer id.
    pub id: u32,
    /// Inbox of received raw frames.
    inbox: Vec<Vec<u8>>,
    /// Whether closed.
    closed: bool,
    /// Next outbound seq.
    next_seq: u32,
}

impl NetPeer {
    /// Create peer.
    pub fn new(id: u32) -> Self {
        Self {
            id,
            inbox: Vec::new(),
            closed: false,
            next_seq: 1,
        }
    }

    /// Send a message to another peer (loopback: pushes into their inbox).
    pub fn send_to(&mut self, other: &mut NetPeer, channel: u16, body: impl Into<String>) -> Result<u32, NetError> {
        if self.closed || other.closed {
            return Err(NetError::Closed);
        }
        let body = body.into();
        if body.is_empty() {
            return Err(NetError::Empty);
        }
        let seq = self.next_seq;
        self.next_seq = self.next_seq.wrapping_add(1);
        let msg = NetMessage::new(channel, seq, body);
        other.inbox.push(msg.encode());
        Ok(seq)
    }

    /// Poll next message (FIFO).
    pub fn poll(&mut self) -> Result<Option<NetMessage>, NetError> {
        if self.closed {
            return Err(NetError::Closed);
        }
        if self.inbox.is_empty() {
            return Ok(None);
        }
        let frame = self.inbox.remove(0);
        Ok(Some(NetMessage::decode(&frame)?))
    }

    /// Close peer.
    pub fn close(&mut self) {
        self.closed = true;
        self.inbox.clear();
    }
}

/// Deterministic loopback: A → B round-trip of one application message.
pub fn loopback_roundtrip(body: &str) -> Result<NetMessage, NetError> {
    let mut a = NetPeer::new(1);
    let mut b = NetPeer::new(2);
    let seq = a.send_to(&mut b, 1, body)?;
    let msg = b
        .poll()?
        .ok_or_else(|| NetError::Decode("no message".into()))?;
    if msg.seq != seq || msg.body != body {
        return Err(NetError::Decode(format!(
            "mismatch seq={} body={}",
            msg.seq, msg.body
        )));
    }
    // echo back
    let _ = b.send_to(&mut a, 1, format!("ack:{}", msg.body))?;
    let ack = a
        .poll()?
        .ok_or_else(|| NetError::Decode("no ack".into()))?;
    if !ack.body.starts_with("ack:") {
        return Err(NetError::Decode("bad ack".into()));
    }
    Ok(msg)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_decode_roundtrip() {
        let m = NetMessage::new(7, 42, "choice:0");
        let bytes = m.encode();
        let d = NetMessage::decode(&bytes).unwrap();
        assert_eq!(m, d);
    }

    #[test]
    fn loopback_delivers_application_message() {
        let msg = loopback_roundtrip("hello-net").unwrap();
        assert_eq!(msg.body, "hello-net");
        assert_eq!(msg.channel, 1);
    }
}
