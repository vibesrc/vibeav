//! Interleaved framing tests.

use vibeav_rtp::interleaved::{
    is_interleaved, peek_frame_size, write_interleaved_header, InterleavedFrame,
    InterleavedFrameBuilder, InterleavedFrameIter, INTERLEAVED_HEADER_SIZE, INTERLEAVED_MAGIC,
};
use vibeav_rtp::InterleavedError;

fn build_frame(channel: u8, data: &[u8]) -> Vec<u8> {
    let mut buf = vec![0u8; INTERLEAVED_HEADER_SIZE + data.len()];
    buf[0] = INTERLEAVED_MAGIC;
    buf[1] = channel;
    buf[2..4].copy_from_slice(&(data.len() as u16).to_be_bytes());
    buf[INTERLEAVED_HEADER_SIZE..].copy_from_slice(data);
    buf
}

#[test]
fn test_parse_interleaved_frame() {
    let data = b"Hello, RTP!";
    let frame_data = build_frame(0, data);

    let (frame, consumed) = InterleavedFrame::parse(&frame_data).unwrap();

    assert_eq!(frame.channel, 0);
    assert_eq!(frame.data, data);
    assert_eq!(consumed, INTERLEAVED_HEADER_SIZE + data.len());
}

#[test]
fn test_parse_rtp_channel() {
    let frame_data = build_frame(0, &[1, 2, 3]);
    let (frame, _) = InterleavedFrame::parse(&frame_data).unwrap();

    assert!(frame.is_rtp());
    assert!(!frame.is_rtcp());
    assert_eq!(frame.rtcp_channel(), Some(1));
    assert_eq!(frame.rtp_channel(), None);
}

#[test]
fn test_parse_rtcp_channel() {
    let frame_data = build_frame(1, &[1, 2, 3]);
    let (frame, _) = InterleavedFrame::parse(&frame_data).unwrap();

    assert!(!frame.is_rtp());
    assert!(frame.is_rtcp());
    assert_eq!(frame.rtp_channel(), Some(0));
    assert_eq!(frame.rtcp_channel(), None);
}

#[test]
fn test_parse_higher_channels() {
    // Channel 2 is RTP, channel 3 is RTCP (second track)
    let frame_data = build_frame(2, &[1, 2, 3]);
    let (frame, _) = InterleavedFrame::parse(&frame_data).unwrap();
    assert!(frame.is_rtp());

    let frame_data = build_frame(3, &[1, 2, 3]);
    let (frame, _) = InterleavedFrame::parse(&frame_data).unwrap();
    assert!(frame.is_rtcp());
}

#[test]
fn test_reject_invalid_magic() {
    let mut frame_data = build_frame(0, &[1, 2, 3]);
    frame_data[0] = 0x00; // Wrong magic

    let err = InterleavedFrame::parse(&frame_data).unwrap_err();
    assert!(matches!(err, InterleavedError::InvalidMagic(0x00)));
}

#[test]
fn test_reject_incomplete_header() {
    let frame_data = [INTERLEAVED_MAGIC, 0, 0]; // Only 3 bytes

    let err = InterleavedFrame::parse(&frame_data).unwrap_err();
    assert!(matches!(err, InterleavedError::IncompleteHeader(3)));
}

#[test]
fn test_reject_incomplete_frame() {
    let mut frame_data = build_frame(0, &[1, 2, 3, 4, 5]);
    // Truncate the data
    frame_data.truncate(6);

    let err = InterleavedFrame::parse(&frame_data).unwrap_err();
    assert!(matches!(err, InterleavedError::IncompleteFrame { .. }));
}

#[test]
fn test_write_header() {
    let mut buf = [0u8; INTERLEAVED_HEADER_SIZE];
    let written = write_interleaved_header(&mut buf, 2, 100);

    assert_eq!(written, INTERLEAVED_HEADER_SIZE);
    assert_eq!(buf[0], INTERLEAVED_MAGIC);
    assert_eq!(buf[1], 2);
    assert_eq!(u16::from_be_bytes([buf[2], buf[3]]), 100);
}

#[test]
fn test_frame_builder() {
    let data = b"Test payload";
    let builder = InterleavedFrameBuilder::new(4);

    let mut buf = [0u8; 64];
    let written = builder.build(&mut buf, data);

    assert_eq!(written, INTERLEAVED_HEADER_SIZE + data.len());

    // Parse it back
    let (frame, consumed) = InterleavedFrame::parse(&buf[..written]).unwrap();
    assert_eq!(frame.channel, 4);
    assert_eq!(frame.data, data);
    assert_eq!(consumed, written);
}

#[test]
fn test_is_interleaved() {
    assert!(is_interleaved(&[INTERLEAVED_MAGIC, 0, 0, 0]));
    assert!(!is_interleaved(&[0x00, 0, 0, 0]));
    assert!(!is_interleaved(&[]));
}

#[test]
fn test_peek_frame_size() {
    let frame = build_frame(0, &[1, 2, 3, 4, 5]);
    assert_eq!(peek_frame_size(&frame), Some(INTERLEAVED_HEADER_SIZE + 5));

    // Incomplete header
    assert_eq!(peek_frame_size(&[INTERLEAVED_MAGIC, 0]), None);

    // Wrong magic
    assert_eq!(peek_frame_size(&[0x00, 0, 0, 5]), None);
}

#[test]
fn test_frame_iterator() {
    let frame1 = build_frame(0, &[1, 2, 3]);
    let frame2 = build_frame(1, &[4, 5, 6, 7]);
    let frame3 = build_frame(2, &[8]);

    let mut compound = frame1;
    compound.extend(frame2);
    compound.extend(frame3);

    let frames: Vec<_> = InterleavedFrameIter::new(&compound)
        .filter_map(|r| r.ok())
        .collect();

    assert_eq!(frames.len(), 3);
    assert_eq!(frames[0].channel, 0);
    assert_eq!(frames[0].data, &[1, 2, 3]);
    assert_eq!(frames[1].channel, 1);
    assert_eq!(frames[1].data, &[4, 5, 6, 7]);
    assert_eq!(frames[2].channel, 2);
    assert_eq!(frames[2].data, &[8]);
}

#[test]
fn test_frame_iterator_partial() {
    let frame1 = build_frame(0, &[1, 2, 3]);
    let mut partial_frame2 = build_frame(1, &[4, 5, 6, 7]);
    partial_frame2.truncate(6); // Truncate second frame

    let mut compound = frame1;
    compound.extend(partial_frame2);

    let mut iter = InterleavedFrameIter::new(&compound);

    // First frame should succeed
    let frame = iter.next().unwrap().unwrap();
    assert_eq!(frame.channel, 0);

    // Second frame is incomplete, iteration stops
    assert!(iter.next().is_none());

    // Remaining data should be available
    assert!(!iter.remaining().is_empty());
}

#[test]
fn test_builder_required_size() {
    assert_eq!(
        InterleavedFrameBuilder::required_size(100),
        INTERLEAVED_HEADER_SIZE + 100
    );
}
