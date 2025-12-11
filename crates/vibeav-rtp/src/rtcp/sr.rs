//! RTCP Sender Report (SR) parsing (RFC 3550 Section 6.4.1).

use super::rr::ReceptionReport;
use super::RtcpHeader;
use crate::error::RtcpError;

/// Sender info section size (20 bytes).
pub const SENDER_INFO_SIZE: usize = 20;

/// Sender information from SR packet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SenderInfo {
    /// NTP timestamp (most significant word).
    pub ntp_sec: u32,
    /// NTP timestamp (least significant word).
    pub ntp_frac: u32,
    /// RTP timestamp corresponding to NTP time.
    pub rtp_timestamp: u32,
    /// Total RTP packets sent.
    pub packet_count: u32,
    /// Total payload bytes sent.
    pub octet_count: u32,
}

impl SenderInfo {
    /// Parse sender info from bytes.
    pub fn parse(data: &[u8]) -> Result<Self, RtcpError> {
        if data.len() < SENDER_INFO_SIZE {
            return Err(RtcpError::PacketTooShort {
                expected: SENDER_INFO_SIZE,
                actual: data.len(),
            });
        }

        Ok(Self {
            ntp_sec: u32::from_be_bytes([data[0], data[1], data[2], data[3]]),
            ntp_frac: u32::from_be_bytes([data[4], data[5], data[6], data[7]]),
            rtp_timestamp: u32::from_be_bytes([data[8], data[9], data[10], data[11]]),
            packet_count: u32::from_be_bytes([data[12], data[13], data[14], data[15]]),
            octet_count: u32::from_be_bytes([data[16], data[17], data[18], data[19]]),
        })
    }

    /// Write sender info to buffer.
    pub fn write(&self, buf: &mut [u8]) -> usize {
        buf[0..4].copy_from_slice(&self.ntp_sec.to_be_bytes());
        buf[4..8].copy_from_slice(&self.ntp_frac.to_be_bytes());
        buf[8..12].copy_from_slice(&self.rtp_timestamp.to_be_bytes());
        buf[12..16].copy_from_slice(&self.packet_count.to_be_bytes());
        buf[16..20].copy_from_slice(&self.octet_count.to_be_bytes());
        SENDER_INFO_SIZE
    }

    /// Get compact NTP timestamp (middle 32 bits) for LSR field.
    pub fn ntp_compact(&self) -> u32 {
        ((self.ntp_sec & 0xFFFF) << 16) | ((self.ntp_frac >> 16) & 0xFFFF)
    }

    /// Get full NTP timestamp as u64.
    pub fn ntp_timestamp(&self) -> u64 {
        ((self.ntp_sec as u64) << 32) | (self.ntp_frac as u64)
    }
}

/// RTCP Sender Report packet.
#[derive(Debug, Clone)]
pub struct RtcpSr<'a> {
    /// SSRC of sender.
    pub ssrc: u32,
    /// Sender information.
    pub sender_info: SenderInfo,
    /// Reception report blocks (raw bytes).
    pub reports_data: &'a [u8],
    /// Number of reception reports.
    pub report_count: u8,
}

impl<'a> RtcpSr<'a> {
    /// Minimum SR packet size (SSRC + sender info).
    pub const MIN_SIZE: usize = 4 + SENDER_INFO_SIZE;

    /// Parse SR packet from payload (after header).
    pub fn parse(header: &RtcpHeader, data: &'a [u8]) -> Result<Self, RtcpError> {
        if data.len() < Self::MIN_SIZE {
            return Err(RtcpError::PacketTooShort {
                expected: Self::MIN_SIZE,
                actual: data.len(),
            });
        }

        let ssrc = u32::from_be_bytes([data[0], data[1], data[2], data[3]]);
        let sender_info = SenderInfo::parse(&data[4..])?;

        let reports_start = 4 + SENDER_INFO_SIZE;
        let reports_data = &data[reports_start..];
        let expected_reports_size = header.count as usize * ReceptionReport::SIZE;

        if reports_data.len() < expected_reports_size {
            return Err(RtcpError::PacketTooShort {
                expected: reports_start + expected_reports_size,
                actual: data.len(),
            });
        }

        Ok(Self {
            ssrc,
            sender_info,
            reports_data: &reports_data[..expected_reports_size],
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
