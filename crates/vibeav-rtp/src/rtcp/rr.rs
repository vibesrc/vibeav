//! RTCP Receiver Report (RR) parsing (RFC 3550 Section 6.4.2).

use super::RtcpHeader;
use crate::error::RtcpError;

/// Reception report block (24 bytes).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReceptionReport {
    /// SSRC of the source being reported.
    pub ssrc: u32,
    /// Fraction of packets lost (0-255, scaled by 256).
    pub fraction_lost: u8,
    /// Cumulative packets lost (signed 24-bit).
    pub cumulative_lost: i32,
    /// Extended highest sequence number received.
    pub extended_seq: u32,
    /// Interarrival jitter.
    pub jitter: u32,
    /// Last SR timestamp (middle 32 bits of NTP).
    pub lsr: u32,
    /// Delay since last SR (units of 1/65536 seconds).
    pub dlsr: u32,
}

impl ReceptionReport {
    /// Reception report block size.
    pub const SIZE: usize = 24;

    /// Parse reception report from bytes.
    pub fn parse(data: &[u8]) -> Result<Self, RtcpError> {
        if data.len() < Self::SIZE {
            return Err(RtcpError::PacketTooShort {
                expected: Self::SIZE,
                actual: data.len(),
            });
        }

        let ssrc = u32::from_be_bytes([data[0], data[1], data[2], data[3]]);
        let fraction_lost = data[4];

        // Cumulative lost is signed 24-bit
        let lost_bytes = [data[5], data[6], data[7]];
        let cumulative_lost = if lost_bytes[0] & 0x80 != 0 {
            // Sign extend from 24-bit
            let raw = u32::from_be_bytes([0xFF, lost_bytes[0], lost_bytes[1], lost_bytes[2]]);
            raw as i32
        } else {
            let raw = u32::from_be_bytes([0x00, lost_bytes[0], lost_bytes[1], lost_bytes[2]]);
            raw as i32
        };

        let extended_seq = u32::from_be_bytes([data[8], data[9], data[10], data[11]]);
        let jitter = u32::from_be_bytes([data[12], data[13], data[14], data[15]]);
        let lsr = u32::from_be_bytes([data[16], data[17], data[18], data[19]]);
        let dlsr = u32::from_be_bytes([data[20], data[21], data[22], data[23]]);

        Ok(Self {
            ssrc,
            fraction_lost,
            cumulative_lost,
            extended_seq,
            jitter,
            lsr,
            dlsr,
        })
    }

    /// Write reception report to buffer.
    pub fn write(&self, buf: &mut [u8]) -> usize {
        buf[0..4].copy_from_slice(&self.ssrc.to_be_bytes());
        buf[4] = self.fraction_lost;

        // Cumulative lost as 24-bit signed
        let lost_u32 = self.cumulative_lost as u32;
        buf[5] = ((lost_u32 >> 16) & 0xFF) as u8;
        buf[6] = ((lost_u32 >> 8) & 0xFF) as u8;
        buf[7] = (lost_u32 & 0xFF) as u8;

        buf[8..12].copy_from_slice(&self.extended_seq.to_be_bytes());
        buf[12..16].copy_from_slice(&self.jitter.to_be_bytes());
        buf[16..20].copy_from_slice(&self.lsr.to_be_bytes());
        buf[20..24].copy_from_slice(&self.dlsr.to_be_bytes());

        Self::SIZE
    }

    /// Get the sequence number cycles (high 16 bits of extended seq).
    pub fn seq_cycles(&self) -> u16 {
        (self.extended_seq >> 16) as u16
    }

    /// Get the highest sequence number received (low 16 bits of extended seq).
    pub fn highest_seq(&self) -> u16 {
        (self.extended_seq & 0xFFFF) as u16
    }

    /// Convert fraction lost to percentage (0.0-100.0).
    pub fn loss_percent(&self) -> f32 {
        (self.fraction_lost as f32 / 256.0) * 100.0
    }

    /// Convert DLSR to seconds.
    pub fn dlsr_seconds(&self) -> f64 {
        self.dlsr as f64 / 65536.0
    }
}

/// RTCP Receiver Report packet.
#[derive(Debug, Clone)]
pub struct RtcpRr<'a> {
    /// SSRC of packet sender.
    pub ssrc: u32,
    /// Reception report blocks (raw bytes).
    pub reports_data: &'a [u8],
    /// Number of reception reports.
    pub report_count: u8,
}

impl<'a> RtcpRr<'a> {
    /// Minimum RR packet size (just SSRC).
    pub const MIN_SIZE: usize = 4;

    /// Parse RR packet from payload (after header).
    pub fn parse(header: &RtcpHeader, data: &'a [u8]) -> Result<Self, RtcpError> {
        if data.len() < Self::MIN_SIZE {
            return Err(RtcpError::PacketTooShort {
                expected: Self::MIN_SIZE,
                actual: data.len(),
            });
        }

        let ssrc = u32::from_be_bytes([data[0], data[1], data[2], data[3]]);
        let reports_data = &data[4..];
        let expected_size = header.count as usize * ReceptionReport::SIZE;

        if reports_data.len() < expected_size {
            return Err(RtcpError::PacketTooShort {
                expected: 4 + expected_size,
                actual: data.len(),
            });
        }

        Ok(Self {
            ssrc,
            reports_data: &reports_data[..expected_size],
            report_count: header.count,
        })
    }

    /// Iterate over reception reports.
    pub fn reports(&self) -> impl Iterator<Item = ReceptionReport> + '_ {
        self.reports_data
            .chunks_exact(ReceptionReport::SIZE)
            .filter_map(|chunk| ReceptionReport::parse(chunk).ok())
    }
}
