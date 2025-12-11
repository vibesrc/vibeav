//! RTCP packet parsing tests.

use vibeav_rtp::rtcp::{
    parse_compound_rtcp, parse_rtcp_packet, validate_compound_rtcp, RtcpByeBuilder, RtcpHeader,
    RtcpPacket, RtcpPacketType, SdesItemType, RTCP_HEADER_SIZE,
};
use vibeav_rtp::RtcpError;

/// Build a minimal RTCP SR packet.
fn build_sr(ssrc: u32, ntp_sec: u32, ntp_frac: u32, rtp_ts: u32) -> Vec<u8> {
    let mut buf = vec![0u8; RTCP_HEADER_SIZE + 24]; // header + SSRC + sender info

    // V=2, P=0, RC=0, PT=200
    buf[0] = 0x80;
    buf[1] = 200;
    // Length = 6 (28 bytes / 4 - 1)
    buf[2..4].copy_from_slice(&6u16.to_be_bytes());
    // SSRC
    buf[4..8].copy_from_slice(&ssrc.to_be_bytes());
    // NTP timestamp
    buf[8..12].copy_from_slice(&ntp_sec.to_be_bytes());
    buf[12..16].copy_from_slice(&ntp_frac.to_be_bytes());
    // RTP timestamp
    buf[16..20].copy_from_slice(&rtp_ts.to_be_bytes());
    // Packet count
    buf[20..24].copy_from_slice(&100u32.to_be_bytes());
    // Octet count
    buf[24..28].copy_from_slice(&10000u32.to_be_bytes());

    buf
}

/// Build a minimal RTCP RR packet.
fn build_rr(ssrc: u32) -> Vec<u8> {
    let mut buf = vec![0u8; RTCP_HEADER_SIZE + 4]; // header + SSRC

    // V=2, P=0, RC=0, PT=201
    buf[0] = 0x80;
    buf[1] = 201;
    // Length = 1 (8 bytes / 4 - 1)
    buf[2..4].copy_from_slice(&1u16.to_be_bytes());
    // SSRC
    buf[4..8].copy_from_slice(&ssrc.to_be_bytes());

    buf
}

/// Build a minimal RTCP SDES packet with CNAME.
fn build_sdes(ssrc: u32, cname: &str) -> Vec<u8> {
    let cname_bytes = cname.as_bytes();
    let items_len = 2 + cname_bytes.len(); // type + length + value
    let chunk_len = 4 + items_len + 1; // SSRC + items + END
    let padded_chunk_len = ((chunk_len + 3) / 4) * 4;
    let total_len = RTCP_HEADER_SIZE + padded_chunk_len;

    let mut buf = vec![0u8; total_len];

    // V=2, P=0, SC=1, PT=202
    buf[0] = 0x81;
    buf[1] = 202;
    // Length
    let length_words = (total_len / 4 - 1) as u16;
    buf[2..4].copy_from_slice(&length_words.to_be_bytes());
    // SSRC
    buf[4..8].copy_from_slice(&ssrc.to_be_bytes());
    // CNAME item
    buf[8] = 1; // CNAME type
    buf[9] = cname_bytes.len() as u8;
    buf[10..10 + cname_bytes.len()].copy_from_slice(cname_bytes);
    // END marker (already 0)

    buf
}

/// Build a minimal RTCP BYE packet.
fn build_bye(ssrcs: &[u32], reason: Option<&str>) -> Vec<u8> {
    let ssrcs_size = ssrcs.len() * 4;
    let reason_size = reason.map_or(0, |r| {
        let len = r.len();
        ((1 + len + 3) / 4) * 4
    });
    let total_len = RTCP_HEADER_SIZE + ssrcs_size + reason_size;

    let mut buf = vec![0u8; total_len];

    // V=2, P=0, SC, PT=203
    buf[0] = 0x80 | (ssrcs.len() as u8);
    buf[1] = 203;
    // Length
    let length_words = (total_len / 4 - 1) as u16;
    buf[2..4].copy_from_slice(&length_words.to_be_bytes());

    // SSRCs
    let mut offset = 4;
    for ssrc in ssrcs {
        buf[offset..offset + 4].copy_from_slice(&ssrc.to_be_bytes());
        offset += 4;
    }

    // Reason
    if let Some(r) = reason {
        buf[offset] = r.len() as u8;
        offset += 1;
        buf[offset..offset + r.len()].copy_from_slice(r.as_bytes());
    }

    buf
}

#[test]
fn test_parse_rtcp_header() {
    let packet = build_sr(0x12345678, 0, 0, 0);

    let (header, size) = RtcpHeader::parse(&packet).unwrap();

    assert_eq!(header.version, 2);
    assert!(!header.padding);
    assert_eq!(header.count, 0);
    assert_eq!(header.packet_type, 200);
    assert_eq!(header.length, 6);
    assert_eq!(size, 28);
}

#[test]
fn test_parse_sr() {
    let packet = build_sr(0xAABBCCDD, 0x12345678, 0x9ABCDEF0, 1000);

    let (rtcp, _) = parse_rtcp_packet(&packet).unwrap();

    if let RtcpPacket::Sr(sr) = rtcp {
        assert_eq!(sr.ssrc, 0xAABBCCDD);
        assert_eq!(sr.sender_info.ntp_sec, 0x12345678);
        assert_eq!(sr.sender_info.ntp_frac, 0x9ABCDEF0);
        assert_eq!(sr.sender_info.rtp_timestamp, 1000);
        assert_eq!(sr.sender_info.packet_count, 100);
        assert_eq!(sr.sender_info.octet_count, 10000);
        assert_eq!(sr.report_count, 0);
    } else {
        panic!("Expected SR packet");
    }
}

#[test]
fn test_parse_rr() {
    let packet = build_rr(0x11223344);

    let (rtcp, _) = parse_rtcp_packet(&packet).unwrap();

    if let RtcpPacket::Rr(rr) = rtcp {
        assert_eq!(rr.ssrc, 0x11223344);
        assert_eq!(rr.report_count, 0);
    } else {
        panic!("Expected RR packet");
    }
}

#[test]
fn test_parse_sdes() {
    let packet = build_sdes(0xDEADBEEF, "alice@example.com");

    let (rtcp, _) = parse_rtcp_packet(&packet).unwrap();

    if let RtcpPacket::Sdes(sdes) = rtcp {
        assert_eq!(sdes.source_count(), 1);

        let chunks: Vec<_> = sdes.chunks().collect();
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].ssrc, 0xDEADBEEF);
        assert_eq!(chunks[0].cname_str().unwrap(), "alice@example.com");
    } else {
        panic!("Expected SDES packet");
    }
}

#[test]
fn test_parse_bye() {
    let packet = build_bye(&[0x11111111, 0x22222222], Some("Goodbye!"));

    let (rtcp, _) = parse_rtcp_packet(&packet).unwrap();

    if let RtcpPacket::Bye(bye) = rtcp {
        assert_eq!(bye.ssrc_count(), 2);

        let ssrcs: Vec<_> = bye.ssrcs().collect();
        assert_eq!(ssrcs, vec![0x11111111, 0x22222222]);
        assert_eq!(bye.reason_str().unwrap(), "Goodbye!");
    } else {
        panic!("Expected BYE packet");
    }
}

#[test]
fn test_parse_bye_no_reason() {
    let packet = build_bye(&[0x12345678], None);

    let (rtcp, _) = parse_rtcp_packet(&packet).unwrap();

    if let RtcpPacket::Bye(bye) = rtcp {
        assert_eq!(bye.ssrc_count(), 1);
        assert!(bye.reason.is_none());
    } else {
        panic!("Expected BYE packet");
    }
}

#[test]
fn test_parse_compound_rtcp() {
    // Build compound packet: SR + SDES
    let sr = build_sr(0xAAAAAAAA, 0, 0, 0);
    let sdes = build_sdes(0xAAAAAAAA, "test@host");

    let mut compound = sr;
    compound.extend(sdes);

    let packets: Vec<_> = parse_compound_rtcp(&compound).collect();
    assert_eq!(packets.len(), 2);

    assert!(matches!(packets[0], Ok(RtcpPacket::Sr(_))));
    assert!(matches!(packets[1], Ok(RtcpPacket::Sdes(_))));
}

#[test]
fn test_reject_invalid_version() {
    let mut packet = build_sr(0, 0, 0, 0);
    // Set version to 1
    packet[0] = (1 << 6) | (packet[0] & 0x3F);

    let err = parse_rtcp_packet(&packet).unwrap_err();
    assert!(matches!(err, RtcpError::InvalidVersion(1)));
}

#[test]
fn test_reject_short_packet() {
    let packet = [0u8; 2];

    let err = parse_rtcp_packet(&packet).unwrap_err();
    assert!(matches!(err, RtcpError::PacketTooShort { .. }));
}

#[test]
fn test_packet_type_from_u8() {
    assert_eq!(RtcpPacketType::from_u8(200), Some(RtcpPacketType::Sr));
    assert_eq!(RtcpPacketType::from_u8(201), Some(RtcpPacketType::Rr));
    assert_eq!(RtcpPacketType::from_u8(202), Some(RtcpPacketType::Sdes));
    assert_eq!(RtcpPacketType::from_u8(203), Some(RtcpPacketType::Bye));
    assert_eq!(RtcpPacketType::from_u8(204), Some(RtcpPacketType::App));
    assert_eq!(RtcpPacketType::from_u8(199), None);
    assert_eq!(RtcpPacketType::from_u8(205), None);
}

#[test]
fn test_bye_builder() {
    let mut buf = [0u8; 64];

    let size = RtcpByeBuilder::new()
        .add_ssrc(0x12345678)
        .add_ssrc(0xDEADBEEF)
        .reason("Leaving session")
        .build(&mut buf);

    // Parse it back
    let (header, _) = RtcpHeader::parse(&buf[..size]).unwrap();
    assert_eq!(header.packet_type, 203);
    assert_eq!(header.count, 2);

    let (rtcp, _) = parse_rtcp_packet(&buf[..size]).unwrap();
    if let RtcpPacket::Bye(bye) = rtcp {
        let ssrcs: Vec<_> = bye.ssrcs().collect();
        assert_eq!(ssrcs, vec![0x12345678, 0xDEADBEEF]);
        assert_eq!(bye.reason_str().unwrap(), "Leaving session");
    } else {
        panic!("Expected BYE packet");
    }
}

#[test]
fn test_sender_info_ntp_compact() {
    use vibeav_rtp::rtcp::SenderInfo;

    let info = SenderInfo {
        ntp_sec: 0x83AA7E80,
        ntp_frac: 0x80000000,
        rtp_timestamp: 0,
        packet_count: 0,
        octet_count: 0,
    };

    // Compact should be middle 32 bits: low 16 of sec + high 16 of frac
    assert_eq!(info.ntp_compact(), 0x7E808000);
}

#[test]
fn test_reception_report_helpers() {
    use vibeav_rtp::rtcp::ReceptionReport;

    let report = ReceptionReport {
        ssrc: 0x12345678,
        fraction_lost: 51, // ~20% loss
        cumulative_lost: 100,
        extended_seq: (5 << 16) | 1234, // 5 cycles, seq 1234
        jitter: 500,
        lsr: 0x12345678,
        dlsr: 65536, // 1 second
    };

    assert_eq!(report.seq_cycles(), 5);
    assert_eq!(report.highest_seq(), 1234);
    assert!((report.loss_percent() - 19.92).abs() < 0.1);
    assert!((report.dlsr_seconds() - 1.0).abs() < 0.001);
}

#[test]
fn test_sr_with_report_blocks() {
    // SR with one reception report block
    let mut buf = vec![0u8; RTCP_HEADER_SIZE + 24 + 24]; // header + sender info + 1 report block

    // V=2, P=0, RC=1, PT=200
    buf[0] = 0x81;
    buf[1] = 200;
    // Length = 12 (52 bytes / 4 - 1)
    buf[2..4].copy_from_slice(&12u16.to_be_bytes());
    // SSRC
    buf[4..8].copy_from_slice(&0x11111111u32.to_be_bytes());
    // NTP timestamp
    buf[8..12].copy_from_slice(&0x12345678u32.to_be_bytes());
    buf[12..16].copy_from_slice(&0x9ABCDEF0u32.to_be_bytes());
    // RTP timestamp
    buf[16..20].copy_from_slice(&1000u32.to_be_bytes());
    // Packet count
    buf[20..24].copy_from_slice(&50u32.to_be_bytes());
    // Octet count
    buf[24..28].copy_from_slice(&5000u32.to_be_bytes());

    // Report block
    buf[28..32].copy_from_slice(&0x22222222u32.to_be_bytes()); // SSRC
    buf[32] = 25; // Fraction lost (~10%)
    buf[33..36].copy_from_slice(&[0x00, 0x00, 0x0A]); // Cumulative lost = 10
    buf[36..40].copy_from_slice(&0x00011234u32.to_be_bytes()); // Extended seq
    buf[40..44].copy_from_slice(&100u32.to_be_bytes()); // Jitter
    buf[44..48].copy_from_slice(&0xABCD1234u32.to_be_bytes()); // LSR
    buf[48..52].copy_from_slice(&0x00010000u32.to_be_bytes()); // DLSR

    let (rtcp, _) = parse_rtcp_packet(&buf).unwrap();
    if let RtcpPacket::Sr(sr) = rtcp {
        assert_eq!(sr.ssrc, 0x11111111);
        assert_eq!(sr.report_count, 1);

        let reports: Vec<_> = sr.reports().collect();
        assert_eq!(reports.len(), 1);
        assert_eq!(reports[0].ssrc, 0x22222222);
        assert_eq!(reports[0].fraction_lost, 25);
        assert_eq!(reports[0].cumulative_lost, 10);
    } else {
        panic!("Expected SR packet");
    }
}

#[test]
fn test_rr_with_report_blocks() {
    // RR with one reception report block
    let mut buf = vec![0u8; RTCP_HEADER_SIZE + 4 + 24]; // header + SSRC + 1 report block

    // V=2, P=0, RC=1, PT=201
    buf[0] = 0x81;
    buf[1] = 201;
    // Length = 7 (32 bytes / 4 - 1)
    buf[2..4].copy_from_slice(&7u16.to_be_bytes());
    // SSRC
    buf[4..8].copy_from_slice(&0x11111111u32.to_be_bytes());

    // Report block
    buf[8..12].copy_from_slice(&0x22222222u32.to_be_bytes()); // SSRC
    buf[12] = 51; // Fraction lost (~20%)
    buf[13..16].copy_from_slice(&[0xFF, 0xFF, 0xF6]); // Cumulative lost = -10 (negative)
    buf[16..20].copy_from_slice(&0x00021234u32.to_be_bytes()); // Extended seq
    buf[20..24].copy_from_slice(&200u32.to_be_bytes()); // Jitter
    buf[24..28].copy_from_slice(&0u32.to_be_bytes()); // LSR
    buf[28..32].copy_from_slice(&0u32.to_be_bytes()); // DLSR

    let (rtcp, _) = parse_rtcp_packet(&buf).unwrap();
    if let RtcpPacket::Rr(rr) = rtcp {
        assert_eq!(rr.ssrc, 0x11111111);
        assert_eq!(rr.report_count, 1);

        let reports: Vec<_> = rr.reports().collect();
        assert_eq!(reports.len(), 1);
        assert_eq!(reports[0].ssrc, 0x22222222);
        assert_eq!(reports[0].cumulative_lost, -10); // Negative (duplicates received)
    } else {
        panic!("Expected RR packet");
    }
}

#[test]
fn test_sdes_multiple_items() {
    // SDES with CNAME and NAME
    let mut buf = vec![0u8; 32];

    // V=2, P=0, SC=1, PT=202
    buf[0] = 0x81;
    buf[1] = 202;
    buf[2..4].copy_from_slice(&7u16.to_be_bytes()); // length
    buf[4..8].copy_from_slice(&0x12345678u32.to_be_bytes()); // SSRC

    // CNAME
    buf[8] = 1; // CNAME
    buf[9] = 4; // length
    buf[10..14].copy_from_slice(b"test");

    // NAME
    buf[14] = 2; // NAME
    buf[15] = 5; // length
    buf[16..21].copy_from_slice(b"Alice");

    // END + padding
    buf[21] = 0;

    let (rtcp, _) = parse_rtcp_packet(&buf).unwrap();
    if let RtcpPacket::Sdes(sdes) = rtcp {
        let chunks: Vec<_> = sdes.chunks().collect();
        assert_eq!(chunks.len(), 1);

        let items: Vec<_> = chunks[0].items().collect();
        assert_eq!(items.len(), 2);
        assert!(items[0].is_cname());
        assert_eq!(items[0].value_str(), "test");
        assert_eq!(items[1].item_type, 2);
        assert_eq!(items[1].value_str(), "Alice");
    } else {
        panic!("Expected SDES packet");
    }
}

#[test]
fn test_sdes_item_types() {
    assert_eq!(SdesItemType::from_u8(0), Some(SdesItemType::End));
    assert_eq!(SdesItemType::from_u8(1), Some(SdesItemType::Cname));
    assert_eq!(SdesItemType::from_u8(2), Some(SdesItemType::Name));
    assert_eq!(SdesItemType::from_u8(3), Some(SdesItemType::Email));
    assert_eq!(SdesItemType::from_u8(4), Some(SdesItemType::Phone));
    assert_eq!(SdesItemType::from_u8(5), Some(SdesItemType::Loc));
    assert_eq!(SdesItemType::from_u8(6), Some(SdesItemType::Tool));
    assert_eq!(SdesItemType::from_u8(7), Some(SdesItemType::Note));
    assert_eq!(SdesItemType::from_u8(8), Some(SdesItemType::Priv));
    assert_eq!(SdesItemType::from_u8(9), None);
}

#[test]
fn test_app_packet() {
    let mut buf = vec![0u8; 16];

    // V=2, P=0, subtype=5, PT=204
    buf[0] = 0x85;
    buf[1] = 204;
    buf[2..4].copy_from_slice(&3u16.to_be_bytes()); // length
    buf[4..8].copy_from_slice(&0x12345678u32.to_be_bytes()); // SSRC
    buf[8..12].copy_from_slice(b"TEST"); // name
    buf[12..16].copy_from_slice(&[0xDE, 0xAD, 0xBE, 0xEF]); // data

    let (rtcp, _) = parse_rtcp_packet(&buf).unwrap();
    if let RtcpPacket::App(app) = rtcp {
        assert_eq!(app.subtype, 5);
        assert_eq!(app.ssrc, 0x12345678);
        assert_eq!(app.name_str(), "TEST");
        assert_eq!(app.data, &[0xDE, 0xAD, 0xBE, 0xEF]);
    } else {
        panic!("Expected APP packet");
    }
}

#[test]
fn test_unknown_packet_type() {
    let mut buf = vec![0u8; 8];

    // V=2, P=0, RC=0, PT=199 (unknown)
    buf[0] = 0x80;
    buf[1] = 199;
    buf[2..4].copy_from_slice(&1u16.to_be_bytes());
    buf[4..8].copy_from_slice(&0x12345678u32.to_be_bytes());

    let (rtcp, _) = parse_rtcp_packet(&buf).unwrap();
    assert!(matches!(rtcp, RtcpPacket::Unknown { packet_type: 199, .. }));
}

#[test]
fn test_validate_compound_rtcp() {
    // Valid compound: starts with SR
    let sr = build_sr(0x12345678, 0, 0, 0);
    assert!(validate_compound_rtcp(&sr).is_ok());

    // Valid compound: starts with RR
    let rr = build_rr(0x12345678);
    assert!(validate_compound_rtcp(&rr).is_ok());

    // Invalid: starts with SDES
    let sdes = build_sdes(0x12345678, "test");
    let err = validate_compound_rtcp(&sdes).unwrap_err();
    assert!(matches!(err, RtcpError::InvalidCompoundStart(202)));
}

#[test]
fn test_header_write() {
    let header = RtcpHeader {
        version: 2,
        padding: true,
        count: 5,
        packet_type: 200,
        length: 0, // length=0 means 4 bytes total (header only)
    };

    let mut buf = [0u8; 4];
    header.write(&mut buf);

    // Check raw bytes directly since parse validates length
    assert_eq!(buf[0], 0xA5); // V=2, P=1, RC=5
    assert_eq!(buf[1], 200);  // PT=200
    assert_eq!(buf[2], 0);    // length high
    assert_eq!(buf[3], 0);    // length low
}

#[test]
fn test_bye_builder_no_reason() {
    let mut buf = [0u8; 16];

    let size = RtcpByeBuilder::new()
        .add_ssrc(0x12345678)
        .build(&mut buf);

    let (rtcp, _) = parse_rtcp_packet(&buf[..size]).unwrap();
    if let RtcpPacket::Bye(bye) = rtcp {
        assert_eq!(bye.ssrc_count(), 1);
        assert!(bye.reason.is_none());
    } else {
        panic!("Expected BYE packet");
    }
}

#[test]
fn test_sender_info_ntp_timestamp() {
    use vibeav_rtp::rtcp::SenderInfo;

    let info = SenderInfo {
        ntp_sec: 0x12345678,
        ntp_frac: 0x9ABCDEF0,
        rtp_timestamp: 0,
        packet_count: 0,
        octet_count: 0,
    };

    assert_eq!(info.ntp_timestamp(), 0x123456789ABCDEF0);
}

#[test]
fn test_sender_info_write() {
    use vibeav_rtp::rtcp::SenderInfo;

    let info = SenderInfo {
        ntp_sec: 0x12345678,
        ntp_frac: 0x9ABCDEF0,
        rtp_timestamp: 1000,
        packet_count: 50,
        octet_count: 5000,
    };

    let mut buf = [0u8; 20];
    info.write(&mut buf);

    let parsed = SenderInfo::parse(&buf).unwrap();
    assert_eq!(parsed.ntp_sec, 0x12345678);
    assert_eq!(parsed.ntp_frac, 0x9ABCDEF0);
    assert_eq!(parsed.rtp_timestamp, 1000);
    assert_eq!(parsed.packet_count, 50);
    assert_eq!(parsed.octet_count, 5000);
}

#[test]
fn test_reception_report_write() {
    use vibeav_rtp::rtcp::ReceptionReport;

    let report = ReceptionReport {
        ssrc: 0x12345678,
        fraction_lost: 25,
        cumulative_lost: -5, // Negative
        extended_seq: 0x00011234,
        jitter: 100,
        lsr: 0xABCD1234,
        dlsr: 0x00010000,
    };

    let mut buf = [0u8; 24];
    report.write(&mut buf);

    let parsed = ReceptionReport::parse(&buf).unwrap();
    assert_eq!(parsed.ssrc, 0x12345678);
    assert_eq!(parsed.fraction_lost, 25);
    assert_eq!(parsed.cumulative_lost, -5);
    assert_eq!(parsed.extended_seq, 0x00011234);
    assert_eq!(parsed.jitter, 100);
    assert_eq!(parsed.lsr, 0xABCD1234);
    assert_eq!(parsed.dlsr, 0x00010000);
}

#[test]
fn test_length_mismatch() {
    let mut buf = vec![0u8; 8];

    // Header says length=10 but buffer only has 8 bytes
    buf[0] = 0x80;
    buf[1] = 200;
    buf[2..4].copy_from_slice(&10u16.to_be_bytes());

    let err = parse_rtcp_packet(&buf).unwrap_err();
    assert!(matches!(err, RtcpError::LengthMismatch { .. }));
}
