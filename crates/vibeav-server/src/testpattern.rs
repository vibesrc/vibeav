//! H.264 test pattern generator.
//!
//! Uses a pre-encoded SMPTE color bars I-frame, packetized per RFC 6184.
//! Zero runtime dependencies - the H.264 bitstream is embedded at compile time.

use std::sync::Arc;
use std::time::Duration;

use bytes::{BufMut, Bytes, BytesMut};
use tokio::sync::broadcast;
use tokio::time::interval;
use tracing::{debug, info, trace, warn};

use vibeav_core::stream::TrackInfo;
use vibeav_core::{Router, TrackType};

/// Pre-encoded H.264 I-frame (SMPTE color bars, 320x240, Baseline profile).
/// Contains SPS, PPS, SEI, and IDR slice.
const H264_IFRAME: &[u8] = include_bytes!("testpattern.h264");

/// H.264 test pattern configuration.
#[derive(Debug, Clone)]
pub struct TestPatternConfig {
    /// Stream path (e.g., "/live/testpattern").
    pub stream_path: String,
    /// Frames per second.
    pub fps: u32,
    /// RTP payload type (dynamic, 96-127).
    pub payload_type: u8,
    /// RTP SSRC.
    pub ssrc: u32,
}

impl Default for TestPatternConfig {
    fn default() -> Self {
        Self {
            stream_path: "/testpattern".to_string(),
            fps: 1,
            payload_type: 96, // Dynamic payload type for H.264
            ssrc: 0x54455354, // "TEST"
        }
    }
}

impl TestPatternConfig {
    #[allow(dead_code)]
    pub fn with_stream_path(mut self, path: impl Into<String>) -> Self {
        self.stream_path = path.into();
        self
    }

    #[allow(dead_code)]
    pub fn with_fps(mut self, fps: u32) -> Self {
        self.fps = fps;
        self
    }
}

/// H.264 test pattern generator.
pub struct TestPatternGenerator {
    config: TestPatternConfig,
    router: Arc<Router>,
    shutdown: broadcast::Receiver<()>,
    /// Parsed NAL units from the embedded H.264 bitstream.
    nal_units: Vec<NalUnit>,
}

/// A single NAL unit extracted from the H.264 bitstream.
struct NalUnit {
    /// NAL unit type (lower 5 bits of first byte).
    nal_type: u8,
    /// Full NAL unit data (including 1-byte NAL header).
    data: Vec<u8>,
}

impl TestPatternGenerator {
    /// Create a new H.264 test pattern generator.
    pub fn new(
        config: TestPatternConfig,
        router: Arc<Router>,
        shutdown: broadcast::Receiver<()>,
    ) -> Self {
        let nal_units = Self::parse_h264_stream(H264_IFRAME);
        debug!(
            nal_count = nal_units.len(),
            "Parsed embedded H.264 bitstream"
        );
        for nal in &nal_units {
            debug!(
                nal_type = nal.nal_type,
                size = nal.data.len(),
                "NAL unit"
            );
        }

        Self {
            config,
            router,
            shutdown,
            nal_units,
        }
    }

    /// Parse Annex B H.264 bitstream into NAL units.
    fn parse_h264_stream(data: &[u8]) -> Vec<NalUnit> {
        let mut nals = Vec::new();
        let mut i = 0;

        while i < data.len() {
            // Find start code (00 00 00 01 or 00 00 01)
            let start = if i + 4 <= data.len()
                && data[i] == 0
                && data[i + 1] == 0
                && data[i + 2] == 0
                && data[i + 3] == 1
            {
                i += 4;
                i
            } else if i + 3 <= data.len()
                && data[i] == 0
                && data[i + 1] == 0
                && data[i + 2] == 1
            {
                i += 3;
                i
            } else {
                i += 1;
                continue;
            };

            // Find next start code or end of data
            let mut end = start;
            while end < data.len() {
                if end + 4 <= data.len()
                    && data[end] == 0
                    && data[end + 1] == 0
                    && data[end + 2] == 0
                    && data[end + 3] == 1
                {
                    break;
                }
                if end + 3 <= data.len()
                    && data[end] == 0
                    && data[end + 1] == 0
                    && data[end + 2] == 1
                {
                    break;
                }
                end += 1;
            }

            if end > start {
                let nal_data = data[start..end].to_vec();
                if !nal_data.is_empty() {
                    let nal_type = nal_data[0] & 0x1F;
                    nals.push(NalUnit {
                        nal_type,
                        data: nal_data,
                    });
                }
            }

            i = end;
        }

        nals
    }

    /// Run the generator (blocks until shutdown).
    pub async fn run(mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        info!(
            stream = %self.config.stream_path,
            fps = self.config.fps,
            nal_units = self.nal_units.len(),
            "Starting H.264 test pattern generator"
        );

        // Create stream
        let stream = match self.router.get_stream(&self.config.stream_path).await {
            Some(s) => s,
            None => self.router.create_stream(&self.config.stream_path).await?,
        };

        // Generate SDP fmtp parameters (SPS/PPS base64)
        let (profile_level_id, sps_pps_b64) = self.extract_parameter_sets();

        // Add track info for H.264
        let fmtp = format!(
            "packetization-mode=1; profile-level-id={}; sprop-parameter-sets={}",
            profile_level_id, sps_pps_b64
        );
        let track_info = TrackInfo {
            track_type: TrackType::Video,
            payload_type: self.config.payload_type,
            clock_rate: 90000,
            encoding: "H264".to_string(),
            parameters: Some(fmtp),
        };
        stream.add_track(track_info).await;

        // Set ourselves as source
        let source_id = format!("testpattern:{}", self.config.stream_path);
        self.router
            .set_source(&self.config.stream_path, &source_id)
            .await?;

        debug!(stream = %self.config.stream_path, "H.264 test pattern source set");

        // Frame timing
        let frame_duration = Duration::from_millis(1000 / self.config.fps as u64);
        let mut frame_interval = interval(frame_duration);
        let clock_rate = 90000u32;
        let timestamp_increment = clock_rate / self.config.fps;

        let mut sequence: u16 = 0;
        let mut timestamp: u32 = 0;
        let mut frame_count: u64 = 0;

        loop {
            tokio::select! {
                _ = self.shutdown.recv() => {
                    info!(stream = %self.config.stream_path, "H.264 test pattern generator shutting down");
                    break;
                }

                _ = frame_interval.tick() => {
                    // Send all NAL units for this frame
                    let packets = self.create_rtp_packets(sequence, timestamp);

                    for packet in &packets {
                        trace!(
                            ts = timestamp,
                            len = packet.len(),
                            "Sending H.264 test pattern packet"
                        );
                        if let Err(e) = self.router.on_rtp(&self.config.stream_path, packet.clone()).await {
                            warn!(error = %e, "Failed to send H.264 test pattern packet");
                        }
                    }

                    sequence = sequence.wrapping_add(packets.len() as u16);
                    timestamp = timestamp.wrapping_add(timestamp_increment);
                    frame_count += 1;

                    if frame_count % 60 == 0 {
                        debug!(
                            stream = %self.config.stream_path,
                            frames = frame_count,
                            "H.264 test pattern running"
                        );
                    }
                }
            }
        }

        // Clear source
        let _ = self.router.clear_source(&self.config.stream_path).await;

        Ok(())
    }

    /// Extract SPS/PPS as base64 for SDP, and profile-level-id.
    fn extract_parameter_sets(&self) -> (String, String) {
        use base64::Engine;
        let encoder = base64::engine::general_purpose::STANDARD;

        let mut sps_b64 = String::new();
        let mut pps_b64 = String::new();
        let mut profile_level_id = String::from("42c01e"); // Default: Baseline, Level 3.0

        for nal in &self.nal_units {
            match nal.nal_type {
                7 => {
                    // SPS
                    sps_b64 = encoder.encode(&nal.data);
                    // Extract profile-level-id from SPS bytes 1-3
                    if nal.data.len() >= 4 {
                        profile_level_id = format!(
                            "{:02x}{:02x}{:02x}",
                            nal.data[1], nal.data[2], nal.data[3]
                        );
                    }
                }
                8 => {
                    // PPS
                    pps_b64 = encoder.encode(&nal.data);
                }
                _ => {}
            }
        }

        // sprop-parameter-sets is comma-separated: SPS,PPS
        let sprop = if !sps_b64.is_empty() && !pps_b64.is_empty() {
            format!("{},{}", sps_b64, pps_b64)
        } else if !sps_b64.is_empty() {
            sps_b64
        } else {
            pps_b64
        };

        (profile_level_id, sprop)
    }

    /// Create RTP packets for all NAL units in the frame.
    /// RFC 6184: RTP Payload Format for H.264 Video.
    fn create_rtp_packets(&self, seq_start: u16, timestamp: u32) -> Vec<Bytes> {
        let mut packets = Vec::new();
        let mut seq = seq_start;
        let max_payload = 1400; // Leave room for RTP header

        let total_nals = self.nal_units.len();

        for (i, nal) in self.nal_units.iter().enumerate() {
            // Skip SEI NALs (type 6) - they bloat the stream with encoder info
            if nal.nal_type == 6 {
                continue;
            }

            let is_last_nal = i == total_nals - 1;

            if nal.data.len() <= max_payload {
                // Single NAL unit packet - NAL fits in one RTP packet
                let marker = is_last_nal; // Marker bit on last packet of frame
                let packet = self.create_single_nal_packet(seq, timestamp, marker, &nal.data);
                packets.push(packet);
                seq = seq.wrapping_add(1);
            } else {
                // Fragmentation Unit A (FU-A) - NAL too large, split it
                let fu_packets =
                    self.create_fu_a_packets(seq, timestamp, is_last_nal, &nal.data, max_payload);
                seq = seq.wrapping_add(fu_packets.len() as u16);
                packets.extend(fu_packets);
            }
        }

        packets
    }

    /// Create a single NAL unit RTP packet.
    /// RFC 6184 Section 5.6: Single NAL Unit Mode
    fn create_single_nal_packet(
        &self,
        seq: u16,
        timestamp: u32,
        marker: bool,
        nal_data: &[u8],
    ) -> Bytes {
        let mut buf = BytesMut::with_capacity(12 + nal_data.len());

        // RTP header
        buf.put_u8(0x80); // V=2, P=0, X=0, CC=0
        let m_pt = if marker { 0x80 } else { 0x00 } | (self.config.payload_type & 0x7F);
        buf.put_u8(m_pt);
        buf.put_u16(seq);
        buf.put_u32(timestamp);
        buf.put_u32(self.config.ssrc);

        // NAL unit (including 1-byte NAL header)
        buf.extend_from_slice(nal_data);

        buf.freeze()
    }

    /// Create Fragmentation Unit A (FU-A) packets for a large NAL.
    /// RFC 6184 Section 5.8: Fragmentation Units (FUs)
    fn create_fu_a_packets(
        &self,
        seq_start: u16,
        timestamp: u32,
        is_last_nal: bool,
        nal_data: &[u8],
        max_payload: usize,
    ) -> Vec<Bytes> {
        let mut packets = Vec::new();

        if nal_data.is_empty() {
            return packets;
        }

        // Extract original NAL header
        let nal_header = nal_data[0];
        let nal_type = nal_header & 0x1F;
        let nri = nal_header & 0x60; // nal_ref_idc bits

        // FU indicator: same as NAL header but with type = 28 (FU-A)
        let fu_indicator = nri | 28;

        // Skip the original NAL header (1 byte), fragment the rest
        let payload = &nal_data[1..];
        let fu_payload_max = max_payload - 2; // RTP overhead: 1 byte FU indicator + 1 byte FU header

        let mut offset = 0;
        let mut seq = seq_start;
        let mut is_first = true;

        while offset < payload.len() {
            let remaining = payload.len() - offset;
            let chunk_size = remaining.min(fu_payload_max);
            let is_last_fragment = offset + chunk_size >= payload.len();

            // FU header: S (1) | E (1) | R (1) | Type (5)
            let mut fu_header: u8 = nal_type;
            if is_first {
                fu_header |= 0x80; // S bit (Start)
            }
            if is_last_fragment {
                fu_header |= 0x40; // E bit (End)
            }

            // Marker bit: set on last fragment of last NAL in frame
            let marker = is_last_fragment && is_last_nal;

            let mut buf = BytesMut::with_capacity(12 + 2 + chunk_size);

            // RTP header
            buf.put_u8(0x80);
            let m_pt = if marker { 0x80 } else { 0x00 } | (self.config.payload_type & 0x7F);
            buf.put_u8(m_pt);
            buf.put_u16(seq);
            buf.put_u32(timestamp);
            buf.put_u32(self.config.ssrc);

            // FU indicator
            buf.put_u8(fu_indicator);

            // FU header
            buf.put_u8(fu_header);

            // Fragment data
            buf.extend_from_slice(&payload[offset..offset + chunk_size]);

            packets.push(buf.freeze());

            offset += chunk_size;
            seq = seq.wrapping_add(1);
            is_first = false;
        }

        packets
    }
}

/// Spawn the H.264 test pattern generator task.
pub fn spawn_test_pattern(
    config: TestPatternConfig,
    router: Arc<Router>,
    shutdown: broadcast::Receiver<()>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let generator = TestPatternGenerator::new(config, router, shutdown);
        if let Err(e) = generator.run().await {
            warn!(error = %e, "H.264 test pattern generator error");
        }
    })
}
