//! Sink management.
//!
//! A sink receives forwarded media packets from streams.
//! Sinks are the output side of the router - media flows to them.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

use bytes::Bytes;
use tokio::sync::mpsc;

use tracing::{debug, trace};

use crate::error::{CoreError, Result};

/// Unique sink identifier.
pub type SinkId = String;

/// A media packet to be forwarded.
#[derive(Debug, Clone)]
pub struct MediaPacket {
    /// RTP payload type.
    pub payload_type: u8,
    /// RTP sequence number.
    pub sequence: u16,
    /// RTP timestamp.
    pub timestamp: u32,
    /// SSRC.
    pub ssrc: u32,
    /// Marker bit.
    pub marker: bool,
    /// Raw packet data (for zero-copy forwarding).
    pub data: Bytes,
}

impl MediaPacket {
    /// Create from raw RTP packet data.
    pub fn from_rtp(data: Bytes) -> Option<Self> {
        if data.len() < 12 {
            return None;
        }

        let payload_type = data[1] & 0x7F;
        let marker = (data[1] >> 7) & 0x01 != 0;
        let sequence = u16::from_be_bytes([data[2], data[3]]);
        let timestamp = u32::from_be_bytes([data[4], data[5], data[6], data[7]]);
        let ssrc = u32::from_be_bytes([data[8], data[9], data[10], data[11]]);

        Some(Self {
            payload_type,
            sequence,
            timestamp,
            ssrc,
            marker,
            data,
        })
    }
}

/// Sink statistics.
#[derive(Debug, Default)]
pub struct SinkStats {
    /// Packets sent to sink.
    pub packets_sent: AtomicU64,
    /// Bytes sent to sink.
    pub bytes_sent: AtomicU64,
    /// Packets dropped (queue full).
    pub packets_dropped: AtomicU64,
}

impl SinkStats {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn record_sent(&self, bytes: usize) {
        self.packets_sent.fetch_add(1, Ordering::Relaxed);
        self.bytes_sent.fetch_add(bytes as u64, Ordering::Relaxed);
    }

    pub fn record_dropped(&self) {
        self.packets_dropped.fetch_add(1, Ordering::Relaxed);
    }

    pub fn snapshot(&self) -> SinkStatsSnapshot {
        SinkStatsSnapshot {
            packets_sent: self.packets_sent.load(Ordering::Relaxed),
            bytes_sent: self.bytes_sent.load(Ordering::Relaxed),
            packets_dropped: self.packets_dropped.load(Ordering::Relaxed),
        }
    }
}

/// Snapshot of sink statistics.
#[derive(Debug, Clone, Default)]
pub struct SinkStatsSnapshot {
    pub packets_sent: u64,
    pub bytes_sent: u64,
    pub packets_dropped: u64,
}

/// Queue behavior when full.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum QueueFullBehavior {
    /// Drop the oldest packet.
    #[default]
    DropOldest,
    /// Drop the newest packet.
    DropNewest,
    /// Block until space is available.
    Block,
}

/// Sink configuration.
#[derive(Debug, Clone)]
pub struct SinkConfig {
    /// Maximum queue size.
    pub queue_size: usize,
    /// Behavior when queue is full.
    pub queue_full_behavior: QueueFullBehavior,
}

impl Default for SinkConfig {
    fn default() -> Self {
        Self {
            queue_size: 256,
            queue_full_behavior: QueueFullBehavior::DropOldest,
        }
    }
}

/// Shared ring buffer for DropOldest behavior.
type SharedRingBuffer = std::sync::Arc<Mutex<VecDeque<MediaPacket>>>;

/// Internal queue implementation.
enum SinkQueue {
    /// Standard mpsc channel (for Block and DropNewest).
    Channel(mpsc::Sender<MediaPacket>),
    /// Ring buffer with notification channel (for DropOldest).
    RingBuffer {
        buffer: SharedRingBuffer,
        capacity: usize,
        /// Notification sender to wake up receiver.
        notify: mpsc::Sender<()>,
    },
}

impl std::fmt::Debug for SinkQueue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SinkQueue::Channel(_) => f.debug_struct("Channel").finish(),
            SinkQueue::RingBuffer { capacity, .. } => {
                f.debug_struct("RingBuffer").field("capacity", capacity).finish()
            }
        }
    }
}

/// A sink receiving media from a stream.
#[derive(Debug)]
pub struct Sink {
    /// Sink ID.
    pub id: SinkId,
    /// Stream being received from.
    pub stream_id: String,
    /// Internal queue.
    queue: SinkQueue,
    /// Statistics.
    stats: SinkStats,
    /// Configuration.
    config: SinkConfig,
}

/// Receiver for DropOldest mode that wraps the ring buffer.
pub struct RingBufferReceiver {
    buffer: SharedRingBuffer,
    notify: mpsc::Receiver<()>,
}

impl RingBufferReceiver {
    /// Receive the next packet, waiting if necessary.
    pub async fn recv(&mut self) -> Option<MediaPacket> {
        loop {
            // Try to get a packet from the buffer
            {
                let mut buf = self.buffer.lock().unwrap();
                let buf_len = buf.len();
                trace!(buf_len, "RingBufferReceiver::recv checking buffer");
                if let Some(packet) = buf.pop_front() {
                    trace!(remaining = buf.len(), "RingBufferReceiver got packet");
                    return Some(packet);
                }
            }
            // Wait for notification that new packet arrived
            trace!("RingBufferReceiver waiting for notification");
            self.notify.recv().await?;
            trace!("RingBufferReceiver got notification");
        }
    }

    /// Try to receive without blocking.
    pub fn try_recv(&mut self) -> Option<MediaPacket> {
        let mut buf = self.buffer.lock().unwrap();
        buf.pop_front()
    }
}

/// Unified receiver enum.
pub enum SinkReceiver {
    Channel(mpsc::Receiver<MediaPacket>),
    RingBuffer(RingBufferReceiver),
}

impl SinkReceiver {
    /// Receive the next packet.
    pub async fn recv(&mut self) -> Option<MediaPacket> {
        match self {
            SinkReceiver::Channel(rx) => rx.recv().await,
            SinkReceiver::RingBuffer(rx) => rx.recv().await,
        }
    }

    /// Try to receive without blocking.
    pub fn try_recv(&mut self) -> Option<MediaPacket> {
        match self {
            SinkReceiver::Channel(rx) => rx.try_recv().ok(),
            SinkReceiver::RingBuffer(rx) => rx.try_recv(),
        }
    }
}

impl Sink {
    /// Create a new sink.
    ///
    /// Returns the sink and a receiver for packets.
    pub fn new(
        id: impl Into<SinkId>,
        stream_id: impl Into<String>,
        config: SinkConfig,
    ) -> (Self, SinkReceiver) {
        let id = id.into();
        let stream_id = stream_id.into();

        match config.queue_full_behavior {
            QueueFullBehavior::DropOldest => {
                // Use shared ring buffer for DropOldest
                let buffer: SharedRingBuffer = std::sync::Arc::new(
                    Mutex::new(VecDeque::with_capacity(config.queue_size))
                );
                let (notify_tx, notify_rx) = mpsc::channel(1);

                let sink = Self {
                    id,
                    stream_id,
                    queue: SinkQueue::RingBuffer {
                        buffer: buffer.clone(),
                        capacity: config.queue_size,
                        notify: notify_tx,
                    },
                    stats: SinkStats::new(),
                    config,
                };

                let receiver = SinkReceiver::RingBuffer(RingBufferReceiver {
                    buffer,
                    notify: notify_rx,
                });

                (sink, receiver)
            }
            _ => {
                // Use standard channel for Block and DropNewest
                let (sender, receiver) = mpsc::channel(config.queue_size);
                let sink = Self {
                    id,
                    stream_id,
                    queue: SinkQueue::Channel(sender),
                    stats: SinkStats::new(),
                    config,
                };
                (sink, SinkReceiver::Channel(receiver))
            }
        }
    }

    /// Create a new sink with mpsc receiver (for backwards compatibility).
    ///
    /// Note: DropOldest behavior will fall back to DropNewest when using this method.
    pub fn new_with_mpsc(
        id: impl Into<SinkId>,
        stream_id: impl Into<String>,
        config: SinkConfig,
    ) -> (Self, mpsc::Receiver<MediaPacket>) {
        let (sender, receiver) = mpsc::channel(config.queue_size);
        let sink = Self {
            id: id.into(),
            stream_id: stream_id.into(),
            queue: SinkQueue::Channel(sender),
            stats: SinkStats::new(),
            config,
        };
        (sink, receiver)
    }

    /// Send a packet to this sink.
    pub async fn send(&self, packet: MediaPacket) -> Result<()> {
        let bytes = packet.data.len();

        match &self.queue {
            SinkQueue::Channel(sender) => {
                match self.config.queue_full_behavior {
                    QueueFullBehavior::Block => {
                        sender.send(packet).await.map_err(|_| CoreError::ChannelClosed)?;
                        self.stats.record_sent(bytes);
                    }
                    QueueFullBehavior::DropNewest | QueueFullBehavior::DropOldest => {
                        // When using channel, DropOldest falls back to DropNewest
                        match sender.try_send(packet) {
                            Ok(()) => self.stats.record_sent(bytes),
                            Err(mpsc::error::TrySendError::Full(_)) => self.stats.record_dropped(),
                            Err(mpsc::error::TrySendError::Closed(_)) => {
                                return Err(CoreError::ChannelClosed)
                            }
                        }
                    }
                }
            }
            SinkQueue::RingBuffer { buffer, capacity, notify } => {
                let mut buf = buffer.lock().unwrap();

                // If full, drop oldest
                if buf.len() >= *capacity {
                    buf.pop_front();
                    self.stats.record_dropped();
                    trace!(capacity, "Ring buffer full, dropped oldest");
                }

                buf.push_back(packet);
                let buf_len = buf.len();
                self.stats.record_sent(bytes);
                drop(buf); // Release lock before notifying

                // Notify receiver (ignore if channel full - receiver will catch up)
                let notify_result = notify.try_send(());
                trace!(buf_len, notify_ok = notify_result.is_ok(), "Sink::send pushed to ring buffer");
            }
        }

        Ok(())
    }

    /// Try to send a packet without blocking.
    pub fn try_send(&self, packet: MediaPacket) -> Result<()> {
        let bytes = packet.data.len();

        match &self.queue {
            SinkQueue::Channel(sender) => {
                match sender.try_send(packet) {
                    Ok(()) => {
                        self.stats.record_sent(bytes);
                        Ok(())
                    }
                    Err(mpsc::error::TrySendError::Full(_)) => {
                        self.stats.record_dropped();
                        Err(CoreError::QueueFull)
                    }
                    Err(mpsc::error::TrySendError::Closed(_)) => Err(CoreError::ChannelClosed),
                }
            }
            SinkQueue::RingBuffer { buffer, capacity, notify } => {
                let mut buf = buffer.lock().unwrap();

                // If full, drop oldest
                if buf.len() >= *capacity {
                    buf.pop_front();
                    self.stats.record_dropped();
                }

                buf.push_back(packet);
                self.stats.record_sent(bytes);
                drop(buf); // Release lock before notifying

                // Notify receiver
                let _ = notify.try_send(());
                Ok(())
            }
        }
    }

    /// Check if the sink channel is closed.
    pub fn is_closed(&self) -> bool {
        match &self.queue {
            SinkQueue::Channel(sender) => sender.is_closed(),
            SinkQueue::RingBuffer { notify, .. } => notify.is_closed(),
        }
    }

    /// Get sink statistics.
    pub fn stats(&self) -> &SinkStats {
        &self.stats
    }
}
