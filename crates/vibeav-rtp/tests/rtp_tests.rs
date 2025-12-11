//! RTP packet parsing tests.

use vibeav_rtp::rtp::{
    OneByteExtensionIter, RtpExtension, RtpHeader, RtpPacket, EXTENSION_PROFILE_ONE_BYTE,
    EXTENSION_PROFILE_TWO_BYTE, RTP_HEADER_SIZE, RTP_VERSION,
};
use vibeav_rtp::RtpError;

/// Build a minimal RTP packet.
fn build_rtp_packet(
    pt: u8,
    seq: u16,
    ts: u32,
    ssrc: u32,
    payload: &[u8],
) -> Vec<u8> {
    let mut buf = vec![0u8; RTP_HEADER_SIZE + payload.len()];

    // V=2, P=0, X=0, CC=0
    buf[0] = (RTP_VERSION << 6) | 0;
    // M=0, PT
    buf[1] = pt & 0x7F;
    // Sequence
    buf[2..4].copy_from_slice(&seq.to_be_bytes());
    // Timestamp
    buf[4..8].copy_from_slice(&ts.to_be_bytes());
    // SSRC
    buf[8..12].copy_from_slice(&ssrc.to_be_bytes());
    // Payload
    buf[RTP_HEADER_SIZE..].copy_from_slice(payload);

    buf
}

#[test]
fn test_parse_minimal_rtp_header() {
    let packet = build_rtp_packet(96, 1234, 0x12345678, 0xDEADBEEF, &[1, 2, 3, 4]);

    let (header, size) = RtpHeader::parse(&packet).unwrap();

    assert_eq!(header.version, 2);
    assert!(!header.padding);
    assert!(!header.extension);
    assert!(!header.marker);
    assert_eq!(header.payload_type, 96);
    assert_eq!(header.sequence, 1234);
    assert_eq!(header.timestamp, 0x12345678);
    assert_eq!(header.ssrc, 0xDEADBEEF);
    assert_eq!(size, RTP_HEADER_SIZE);
}

#[test]
fn test_parse_rtp_packet_with_payload() {
    let payload = b"Hello, RTP!";
    let packet = build_rtp_packet(96, 100, 1000, 0x11223344, payload);

    let rtp = RtpPacket::parse(&packet).unwrap();

    assert_eq!(rtp.header.payload_type, 96);
    assert_eq!(rtp.header.sequence, 100);
    assert_eq!(rtp.header.timestamp, 1000);
    assert_eq!(rtp.header.ssrc, 0x11223344);
    assert_eq!(rtp.payload, payload);
    assert_eq!(rtp.csrc_count(), 0);
    assert_eq!(rtp.padding_len, 0);
}

#[test]
fn test_parse_rtp_with_marker() {
    let mut packet = build_rtp_packet(96, 1, 0, 0, &[0]);
    // Set marker bit
    packet[1] |= 0x80;

    let rtp = RtpPacket::parse(&packet).unwrap();
    assert!(rtp.header.marker);
}

#[test]
fn test_parse_rtp_with_padding() {
    let mut packet = build_rtp_packet(96, 1, 0, 0, &[0xAA, 0xBB, 0x00, 0x00, 0x04]);
    // Set padding bit
    packet[0] |= 0x20;

    let rtp = RtpPacket::parse(&packet).unwrap();
    assert!(rtp.header.padding);
    assert_eq!(rtp.padding_len, 4);
    // Payload should exclude padding
    assert_eq!(rtp.payload, &[0xAA]);
}

#[test]
fn test_parse_rtp_with_csrc() {
    let mut packet = vec![0u8; RTP_HEADER_SIZE + 8 + 4]; // 2 CSRCs + payload

    // V=2, P=0, X=0, CC=2
    packet[0] = (RTP_VERSION << 6) | 2;
    packet[1] = 96;
    packet[2..4].copy_from_slice(&1u16.to_be_bytes());
    packet[4..8].copy_from_slice(&1000u32.to_be_bytes());
    packet[8..12].copy_from_slice(&0xAAAAAAAAu32.to_be_bytes());
    // CSRC 1
    packet[12..16].copy_from_slice(&0x11111111u32.to_be_bytes());
    // CSRC 2
    packet[16..20].copy_from_slice(&0x22222222u32.to_be_bytes());
    // Payload
    packet[20..24].copy_from_slice(&[1, 2, 3, 4]);

    let rtp = RtpPacket::parse(&packet).unwrap();

    assert_eq!(rtp.csrc_count(), 2);
    let csrcs: Vec<u32> = rtp.csrc_list().collect();
    assert_eq!(csrcs, vec![0x11111111, 0x22222222]);
    assert_eq!(rtp.payload, &[1, 2, 3, 4]);
}

#[test]
fn test_parse_rtp_with_extension() {
    let mut packet = vec![0u8; RTP_HEADER_SIZE + 8 + 4]; // ext header + 4 bytes ext + payload

    // V=2, P=0, X=1, CC=0
    packet[0] = (RTP_VERSION << 6) | 0x10;
    packet[1] = 96;
    packet[2..4].copy_from_slice(&1u16.to_be_bytes());
    packet[4..8].copy_from_slice(&1000u32.to_be_bytes());
    packet[8..12].copy_from_slice(&0xAAAAAAAAu32.to_be_bytes());

    // Extension header: profile=0x1234, length=1 (4 bytes)
    packet[12..14].copy_from_slice(&0x1234u16.to_be_bytes());
    packet[14..16].copy_from_slice(&1u16.to_be_bytes());
    // Extension data
    packet[16..20].copy_from_slice(&[0xDE, 0xAD, 0xBE, 0xEF]);
    // Payload
    packet[20..24].copy_from_slice(&[1, 2, 3, 4]);

    let rtp = RtpPacket::parse(&packet).unwrap();

    assert!(rtp.header.extension);
    let ext = rtp.extension.unwrap();
    assert_eq!(ext.profile, 0x1234);
    assert_eq!(ext.data, &[0xDE, 0xAD, 0xBE, 0xEF]);
    assert_eq!(rtp.payload, &[1, 2, 3, 4]);
}

#[test]
fn test_parse_rtp_one_byte_extension() {
    let mut packet = vec![0u8; RTP_HEADER_SIZE + 8 + 4];

    // V=2, P=0, X=1, CC=0
    packet[0] = (RTP_VERSION << 6) | 0x10;
    packet[1] = 96;
    packet[2..4].copy_from_slice(&1u16.to_be_bytes());
    packet[4..8].copy_from_slice(&1000u32.to_be_bytes());
    packet[8..12].copy_from_slice(&0xAAAAAAAAu32.to_be_bytes());

    // One-byte extension profile: 0xBEDE
    packet[12..14].copy_from_slice(&0xBEDEu16.to_be_bytes());
    packet[14..16].copy_from_slice(&1u16.to_be_bytes()); // 4 bytes
    // Extension element: ID=1, L=1 (2 bytes), value=0xAB, 0xCD
    packet[16] = 0x11; // ID=1, L=1 (means 2 bytes)
    packet[17..19].copy_from_slice(&[0xAB, 0xCD]);
    packet[19] = 0x00; // Padding
    // Payload
    packet[20..24].copy_from_slice(&[1, 2, 3, 4]);

    let rtp = RtpPacket::parse(&packet).unwrap();
    assert!(rtp.is_one_byte_extension());

    let ext = rtp.extension.unwrap();
    let elements: Vec<_> = ext.one_byte_elements().unwrap().collect();
    assert_eq!(elements.len(), 1);
    assert_eq!(elements[0].id, 1);
    assert_eq!(elements[0].data, &[0xAB, 0xCD]);
}

#[test]
fn test_reject_invalid_version() {
    let mut packet = build_rtp_packet(96, 1, 0, 0, &[0]);
    // Set version to 1
    packet[0] = (1 << 6) | (packet[0] & 0x3F);

    let err = RtpPacket::parse(&packet).unwrap_err();
    assert!(matches!(err, RtpError::InvalidVersion(1)));
}

#[test]
fn test_reject_short_packet() {
    let packet = [0u8; 8]; // Too short

    let err = RtpPacket::parse(&packet).unwrap_err();
    assert!(matches!(err, RtpError::PacketTooShort { expected: 12, actual: 8 }));
}

#[test]
fn test_reject_invalid_padding() {
    let mut packet = build_rtp_packet(96, 1, 0, 0, &[0xAA, 0x00]); // padding=0 is invalid
    packet[0] |= 0x20; // Set padding bit

    let err = RtpPacket::parse(&packet).unwrap_err();
    assert!(matches!(err, RtpError::InvalidPadding(0)));
}

#[test]
fn test_header_write() {
    let header = RtpHeader {
        version: 2,
        padding: false,
        extension: false,
        marker: true,
        payload_type: 96,
        sequence: 1234,
        timestamp: 0x12345678,
        ssrc: 0xDEADBEEF,
    };

    let mut buf = [0u8; 12];
    let written = header.write(0, &mut buf);

    assert_eq!(written, 12);

    // Parse it back
    let (parsed, _) = RtpHeader::parse(&buf).unwrap();
    assert_eq!(parsed.version, 2);
    assert!(parsed.marker);
    assert_eq!(parsed.payload_type, 96);
    assert_eq!(parsed.sequence, 1234);
    assert_eq!(parsed.timestamp, 0x12345678);
    assert_eq!(parsed.ssrc, 0xDEADBEEF);
}

#[test]
fn test_header_write_with_padding_and_extension() {
    let header = RtpHeader {
        version: 2,
        padding: true,
        extension: true,
        marker: false,
        payload_type: 111,
        sequence: 65535,
        timestamp: 0xFFFFFFFF,
        ssrc: 0x12345678,
    };

    let mut buf = [0u8; 12];
    header.write(0, &mut buf); // CC=0

    // Check raw bytes since parsing validates extension data exists
    assert_eq!(buf[0], 0xB0); // V=2, P=1, X=1, CC=0
    assert_eq!(buf[1], 111);  // M=0, PT=111
    assert_eq!(u16::from_be_bytes([buf[2], buf[3]]), 65535);
    assert_eq!(u32::from_be_bytes([buf[4], buf[5], buf[6], buf[7]]), 0xFFFFFFFF);
    assert_eq!(u32::from_be_bytes([buf[8], buf[9], buf[10], buf[11]]), 0x12345678);
}

#[test]
fn test_two_byte_extension() {
    let mut packet = vec![0u8; RTP_HEADER_SIZE + 8 + 4];

    // V=2, P=0, X=1, CC=0
    packet[0] = (RTP_VERSION << 6) | 0x10;
    packet[1] = 96;
    packet[2..4].copy_from_slice(&1u16.to_be_bytes());
    packet[4..8].copy_from_slice(&1000u32.to_be_bytes());
    packet[8..12].copy_from_slice(&0xAAAAAAAAu32.to_be_bytes());

    // Two-byte extension profile: 0x1000
    packet[12..14].copy_from_slice(&EXTENSION_PROFILE_TWO_BYTE.to_be_bytes());
    packet[14..16].copy_from_slice(&1u16.to_be_bytes()); // 4 bytes
    packet[16..20].copy_from_slice(&[0x01, 0x02, 0x03, 0x04]);
    packet[20..24].copy_from_slice(&[1, 2, 3, 4]);

    let rtp = RtpPacket::parse(&packet).unwrap();
    assert!(rtp.is_two_byte_extension());
    assert!(!rtp.is_one_byte_extension());

    // one_byte_elements should return None for two-byte extension
    let ext = rtp.extension.unwrap();
    assert!(ext.one_byte_elements().is_none());
}

#[test]
fn test_extension_overflow_short_header() {
    let mut packet = vec![0u8; RTP_HEADER_SIZE + 2]; // Too short for extension header

    // V=2, P=0, X=1, CC=0
    packet[0] = (RTP_VERSION << 6) | 0x10;
    packet[1] = 96;
    packet[2..4].copy_from_slice(&1u16.to_be_bytes());
    packet[4..8].copy_from_slice(&0u32.to_be_bytes());
    packet[8..12].copy_from_slice(&0u32.to_be_bytes());
    // Only 2 bytes after header, but extension needs 4

    let err = RtpPacket::parse(&packet).unwrap_err();
    assert!(matches!(err, RtpError::ExtensionOverflow));
}

#[test]
fn test_extension_overflow_short_data() {
    let mut packet = vec![0u8; RTP_HEADER_SIZE + 6]; // Extension header says 8 bytes but only 2 available

    // V=2, P=0, X=1, CC=0
    packet[0] = (RTP_VERSION << 6) | 0x10;
    packet[1] = 96;
    packet[2..4].copy_from_slice(&1u16.to_be_bytes());
    packet[4..8].copy_from_slice(&0u32.to_be_bytes());
    packet[8..12].copy_from_slice(&0u32.to_be_bytes());

    // Extension header: profile=0x1234, length=2 (8 bytes needed)
    packet[12..14].copy_from_slice(&0x1234u16.to_be_bytes());
    packet[14..16].copy_from_slice(&2u16.to_be_bytes());

    let err = RtpPacket::parse(&packet).unwrap_err();
    assert!(matches!(err, RtpError::ExtensionOverflow));
}

#[test]
fn test_csrc_overflow() {
    let mut packet = vec![0u8; RTP_HEADER_SIZE + 4]; // Says 2 CSRCs but only room for 1

    // V=2, P=0, X=0, CC=2
    packet[0] = (RTP_VERSION << 6) | 2;
    packet[1] = 96;
    packet[2..4].copy_from_slice(&1u16.to_be_bytes());
    packet[4..8].copy_from_slice(&0u32.to_be_bytes());
    packet[8..12].copy_from_slice(&0u32.to_be_bytes());
    // Only 4 bytes after header, but CC=2 needs 8

    let err = RtpPacket::parse(&packet).unwrap_err();
    assert!(matches!(err, RtpError::CsrcOverflow { .. }));
}

#[test]
fn test_padding_too_large() {
    let mut packet = build_rtp_packet(96, 1, 0, 0, &[0xAA, 0xBB, 0x10]); // padding=16, but only 3 bytes
    packet[0] |= 0x20; // Set padding bit

    let err = RtpPacket::parse(&packet).unwrap_err();
    assert!(matches!(err, RtpError::InvalidPadding(16)));
}

#[test]
fn test_one_byte_extension_with_padding() {
    // Test extension iterator skipping padding bytes
    let ext_data = [0x00, 0x00, 0x21, 0xAB, 0xCD]; // 2 padding bytes, then ID=2, L=1, data

    let iter = OneByteExtensionIter::new(&ext_data);
    let elements: Vec<_> = iter.collect();

    assert_eq!(elements.len(), 1);
    assert_eq!(elements[0].id, 2);
    assert_eq!(elements[0].data, &[0xAB, 0xCD]);
}

#[test]
fn test_one_byte_extension_terminator() {
    // ID=15 is terminator
    let ext_data = [0x11, 0xAB, 0xF0]; // ID=1, data, then ID=15 (terminator)

    let iter = OneByteExtensionIter::new(&ext_data);
    let elements: Vec<_> = iter.collect();

    assert_eq!(elements.len(), 1);
    assert_eq!(elements[0].id, 1);
}

#[test]
fn test_one_byte_extension_truncated() {
    // Extension element says 4 bytes but data is truncated
    let ext_data = [0x13, 0xAB]; // ID=1, L=3 (4 bytes needed), but only 1 available

    let iter = OneByteExtensionIter::new(&ext_data);
    let elements: Vec<_> = iter.collect();

    assert_eq!(elements.len(), 0); // Should stop, not panic
}

#[test]
fn test_one_byte_extension_multiple_elements() {
    // Multiple extension elements
    let ext_data = [
        0x10, 0xAA, // ID=1, L=0 (1 byte)
        0x21, 0xBB, 0xCC, // ID=2, L=1 (2 bytes)
        0x30, 0xDD, // ID=3, L=0 (1 byte)
    ];

    let iter = OneByteExtensionIter::new(&ext_data);
    let elements: Vec<_> = iter.collect();

    assert_eq!(elements.len(), 3);
    assert_eq!(elements[0].id, 1);
    assert_eq!(elements[0].data, &[0xAA]);
    assert_eq!(elements[1].id, 2);
    assert_eq!(elements[1].data, &[0xBB, 0xCC]);
    assert_eq!(elements[2].id, 3);
    assert_eq!(elements[2].data, &[0xDD]);
}

#[test]
fn test_no_extension() {
    let packet = build_rtp_packet(96, 1, 0, 0, &[1, 2, 3, 4]);
    let rtp = RtpPacket::parse(&packet).unwrap();

    assert!(!rtp.is_one_byte_extension());
    assert!(!rtp.is_two_byte_extension());
    assert!(rtp.extension.is_none());
}

#[test]
fn test_empty_payload() {
    let packet = build_rtp_packet(96, 1, 0, 0, &[]);
    let rtp = RtpPacket::parse(&packet).unwrap();

    assert!(rtp.payload.is_empty());
}

#[test]
fn test_max_csrc_count() {
    // CC=15 (max)
    let mut packet = vec![0u8; RTP_HEADER_SIZE + 15 * 4 + 4]; // 15 CSRCs + payload

    packet[0] = (RTP_VERSION << 6) | 15;
    packet[1] = 96;
    packet[2..4].copy_from_slice(&1u16.to_be_bytes());
    packet[4..8].copy_from_slice(&0u32.to_be_bytes());
    packet[8..12].copy_from_slice(&0u32.to_be_bytes());

    // Fill 15 CSRCs
    for i in 0..15 {
        let offset = 12 + i * 4;
        packet[offset..offset + 4].copy_from_slice(&(i as u32).to_be_bytes());
    }

    let rtp = RtpPacket::parse(&packet).unwrap();
    assert_eq!(rtp.csrc_count(), 15);

    let csrcs: Vec<_> = rtp.csrc_list().collect();
    assert_eq!(csrcs.len(), 15);
    for (i, &csrc) in csrcs.iter().enumerate() {
        assert_eq!(csrc, i as u32);
    }
}
