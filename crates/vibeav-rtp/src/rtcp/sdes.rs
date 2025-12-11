//! RTCP Source Description (SDES) parsing (RFC 3550 Section 6.5).

use super::RtcpHeader;
use crate::error::RtcpError;

/// SDES item types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum SdesItemType {
    /// End of SDES item list.
    End = 0,
    /// Canonical name (required).
    Cname = 1,
    /// User name.
    Name = 2,
    /// Email address.
    Email = 3,
    /// Phone number.
    Phone = 4,
    /// Geographic location.
    Loc = 5,
    /// Application/tool name.
    Tool = 6,
    /// Status note.
    Note = 7,
    /// Private extension.
    Priv = 8,
}

impl SdesItemType {
    /// Try to convert from u8.
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::End),
            1 => Some(Self::Cname),
            2 => Some(Self::Name),
            3 => Some(Self::Email),
            4 => Some(Self::Phone),
            5 => Some(Self::Loc),
            6 => Some(Self::Tool),
            7 => Some(Self::Note),
            8 => Some(Self::Priv),
            _ => None,
        }
    }
}

/// SDES item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SdesItem<'a> {
    /// Item type.
    pub item_type: u8,
    /// Item value (UTF-8 text).
    pub value: &'a [u8],
}

impl<'a> SdesItem<'a> {
    /// Get value as string (lossy UTF-8 conversion).
    pub fn value_str(&self) -> std::borrow::Cow<'_, str> {
        String::from_utf8_lossy(self.value)
    }

    /// Get the item type as enum.
    pub fn item_type_enum(&self) -> Option<SdesItemType> {
        SdesItemType::from_u8(self.item_type)
    }

    /// Check if this is a CNAME item.
    pub fn is_cname(&self) -> bool {
        self.item_type == SdesItemType::Cname as u8
    }
}

/// SDES chunk (SSRC + items).
#[derive(Debug, Clone)]
pub struct SdesChunk<'a> {
    /// SSRC/CSRC identifier.
    pub ssrc: u32,
    /// Raw items data.
    items_data: &'a [u8],
}

impl<'a> SdesChunk<'a> {
    /// Iterate over items in this chunk.
    pub fn items(&self) -> SdesItemIter<'a> {
        SdesItemIter::new(self.items_data)
    }

    /// Get the CNAME item if present.
    pub fn cname(&self) -> Option<&'a [u8]> {
        self.items()
            .find(|item| item.item_type == SdesItemType::Cname as u8)
            .map(|item| item.value)
    }

    /// Get the CNAME as a string if present.
    pub fn cname_str(&self) -> Option<std::borrow::Cow<'a, str>> {
        self.cname().map(String::from_utf8_lossy)
    }
}

/// Iterator over SDES items.
pub struct SdesItemIter<'a> {
    data: &'a [u8],
    offset: usize,
}

impl<'a> SdesItemIter<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self { data, offset: 0 }
    }
}

impl<'a> Iterator for SdesItemIter<'a> {
    type Item = SdesItem<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.offset >= self.data.len() {
            return None;
        }

        let item_type = self.data[self.offset];

        // Type 0 is END marker
        if item_type == 0 {
            return None;
        }

        // Need at least type + length
        if self.offset + 1 >= self.data.len() {
            return None;
        }

        let length = self.data[self.offset + 1] as usize;
        let value_start = self.offset + 2;
        let value_end = value_start + length;

        if value_end > self.data.len() {
            return None;
        }

        let item = SdesItem {
            item_type,
            value: &self.data[value_start..value_end],
        };

        self.offset = value_end;
        Some(item)
    }
}

/// RTCP SDES packet.
#[derive(Debug, Clone)]
pub struct RtcpSdes<'a> {
    /// Raw packet data (after header).
    data: &'a [u8],
    /// Number of chunks (SC field).
    source_count: u8,
}

impl<'a> RtcpSdes<'a> {
    /// Parse SDES packet from payload (after header).
    pub fn parse(header: &RtcpHeader, data: &'a [u8]) -> Result<Self, RtcpError> {
        Ok(Self {
            data,
            source_count: header.count,
        })
    }

    /// Get the number of SSRC/CSRC chunks.
    pub fn source_count(&self) -> u8 {
        self.source_count
    }

    /// Iterate over SDES chunks.
    pub fn chunks(&self) -> SdesChunkIter<'a> {
        SdesChunkIter {
            data: self.data,
            offset: 0,
            remaining: self.source_count,
        }
    }

    /// Get all CNAMEs in this SDES packet.
    pub fn cnames(&self) -> impl Iterator<Item = (u32, std::borrow::Cow<'a, str>)> + '_ {
        self.chunks().filter_map(|chunk| {
            chunk.cname_str().map(|cname| (chunk.ssrc, cname))
        })
    }
}

/// Iterator over SDES chunks.
pub struct SdesChunkIter<'a> {
    data: &'a [u8],
    offset: usize,
    remaining: u8,
}

impl<'a> Iterator for SdesChunkIter<'a> {
    type Item = SdesChunk<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.remaining == 0 || self.offset + 4 > self.data.len() {
            return None;
        }

        let ssrc = u32::from_be_bytes([
            self.data[self.offset],
            self.data[self.offset + 1],
            self.data[self.offset + 2],
            self.data[self.offset + 3],
        ]);

        self.offset += 4;
        let items_start = self.offset;

        // Find end of items (type=0 or end of data)
        while self.offset < self.data.len() {
            let item_type = self.data[self.offset];

            if item_type == 0 {
                // Found END marker, skip to 32-bit boundary
                self.offset += 1;
                while self.offset % 4 != 0 && self.offset < self.data.len() {
                    self.offset += 1;
                }
                break;
            }

            // Skip this item
            if self.offset + 1 >= self.data.len() {
                break;
            }

            let length = self.data[self.offset + 1] as usize;
            self.offset += 2 + length;
        }

        self.remaining -= 1;

        Some(SdesChunk {
            ssrc,
            items_data: &self.data[items_start..self.offset.min(self.data.len())],
        })
    }
}
