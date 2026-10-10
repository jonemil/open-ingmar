//! Bounded validation before archive backends allocate from header counts.
use crate::{Error, Limits, Result};

struct Header<'a> {
    data: &'a [u8],
    at: usize,
}
impl<'a> Header<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8]> {
        let end = self
            .at
            .checked_add(n)
            .ok_or_else(|| Error::new("header overflow"))?;
        let out = self
            .data
            .get(self.at..end)
            .ok_or_else(|| Error::new("truncated archive header"))?;
        self.at = end;
        Ok(out)
    }
    fn byte(&mut self) -> Result<u8> {
        Ok(self.take(1)?[0])
    }
    fn expect(&mut self, n: u8) -> Result<()> {
        if self.byte()? != n {
            return Err(Error::new("unsupported 7z header structure"));
        }
        Ok(())
    }
    fn number(&mut self) -> Result<u64> {
        let first = self.byte()?;
        let mut mask = 0x80;
        let mut value = 0;
        for shift in 0..8 {
            if first & mask == 0 {
                return Ok(value | (u64::from(first & (mask - 1)) << (shift * 8)));
            }
            value |= u64::from(self.byte()?) << (shift * 8);
            mask >>= 1;
        }
        Ok(value)
    }
    fn count(&mut self, max: usize) -> Result<usize> {
        let n = self.number()?;
        if n > max as u64 {
            return Err(Error::new("archive header count exceeds limit"));
        }
        Ok(n as usize)
    }
    fn bits(&mut self, n: usize) -> Result<usize> {
        let b = self.take(n.div_ceil(8))?;
        Ok((0..n).filter(|i| b[i / 8] & (0x80 >> (i % 8)) != 0).count())
    }
    fn defined(&mut self, n: usize) -> Result<usize> {
        match self.byte()? {
            0 => self.bits(n),
            1 => Ok(n),
            _ => Err(Error::new("invalid 7z bitmap")),
        }
    }
    fn crcs(&mut self, n: usize) -> Result<()> {
        let count = self.defined(n)?;
        self.take(count * 4)?;
        Ok(())
    }
    fn done(&self) -> Result<()> {
        if self.at != self.data.len() {
            return Err(Error::new("unexpected archive header bytes"));
        }
        Ok(())
    }
}

pub(crate) fn zip(data: &[u8], limits: Limits) -> Result<usize> {
    // ZIP64 needs separate bounded preflight before enabling its backend path.
    let start = data.len().saturating_sub(22 + 65535);
    let at = (start..data.len().saturating_sub(21))
        .rev()
        .find(|&i| {
            data.get(i..i + 4) == Some(b"PK\x05\x06")
                && data.get(i + 20..i + 22).is_some_and(|b| {
                    i + 22 + u16::from_le_bytes([b[0], b[1]]) as usize == data.len()
                })
        })
        .ok_or_else(|| Error::new("missing ZIP end directory"))?;
    let u16at = |n| u16::from_le_bytes([data[at + n], data[at + n + 1]]);
    if u16at(4) != 0 || u16at(6) != 0 || u16at(8) != u16at(10) {
        return Err(Error::new("split ZIP is unsupported"));
    }
    let count = u16at(10);
    if count == u16::MAX {
        return Err(Error::new("ZIP64 is unsupported"));
    }
    if count as usize > limits.entries {
        return Err(Error::new("archive entry count exceeds limit"));
    }
    let size = u32::from_le_bytes(data[at + 12..at + 16].try_into().unwrap()) as usize;
    let offset = u32::from_le_bytes(data[at + 16..at + 20].try_into().unwrap()) as usize;
    if size > limits.header_bytes || offset.checked_add(size) != Some(at) {
        return Err(Error::new("invalid or oversized ZIP directory"));
    }
    // Count actual records: don't let a false EOCD count bypass the limit.
    let mut h = Header {
        data: &data[offset..at],
        at: 0,
    };
    let mut actual = 0;
    while h.at < h.data.len() {
        let fixed = h.take(46)?;
        if &fixed[..4] != b"PK\x01\x02" {
            return Err(Error::new("unsupported ZIP directory record"));
        }
        let width = |n| u16::from_le_bytes([fixed[n], fixed[n + 1]]) as usize;
        if fixed[20..28].chunks_exact(4).any(|b| b == [255; 4]) {
            return Err(Error::new("ZIP64 is unsupported"));
        }
        h.take(width(28) + width(30) + width(32))?;
        actual += 1;
        if actual > limits.entries {
            return Err(Error::new("archive entry count exceeds limit"));
        }
    }
    if actual != count as usize {
        return Err(Error::new("ZIP entry count mismatch"));
    }
    Ok(actual)
}

fn dictionary(method: &[u8], properties: &[u8], limits: Limits) -> Result<()> {
    let size = match method {
        [0] if properties.is_empty() => 0,
        [0x21] if properties.len() == 1 && properties[0] <= 40 => {
            let p = properties[0];
            if p == 40 {
                u64::from(u32::MAX)
            } else {
                u64::from(2 | (p & 1)) << (p / 2 + 11)
            }
        }
        [3, 1, 1] if properties.len() == 5 => {
            u64::from(u32::from_le_bytes(properties[1..].try_into().unwrap()))
        }
        _ => return Err(Error::new("unsupported or encrypted 7z coder")),
    };
    if size > limits.dictionary_bytes {
        return Err(Error::new("7z dictionary exceeds limit"));
    }
    Ok(())
}

fn streams(h: &mut Header<'_>, limits: Limits, payload_end: usize) -> Result<usize> {
    h.expect(6)?;
    let position = h.number()?;
    let mut packed_end = position
        .checked_add(32)
        .ok_or_else(|| Error::new("7z packed offset overflow"))?;
    let packed = h.count(limits.entries)?;
    h.expect(9)?;
    for _ in 0..packed {
        packed_end = packed_end
            .checked_add(h.number()?)
            .ok_or_else(|| Error::new("7z packed size overflow"))?;
    }
    if packed_end > payload_end as u64 {
        return Err(Error::new(
            "7z packed streams overlap header or exceed archive",
        ));
    }
    let mut id = h.byte()?;
    if id == 10 {
        h.crcs(packed)?;
        id = h.byte()?;
    }
    if id != 0 {
        return Err(Error::new("invalid 7z pack info"));
    }
    h.expect(7)?;
    h.expect(11)?;
    let blocks = h.count(limits.entries)?;
    h.expect(0)?; // external folder descriptions unsupported
    if blocks != packed {
        return Err(Error::new("unsupported 7z stream layout"));
    }
    for _ in 0..blocks {
        if h.count(1)? != 1 {
            return Err(Error::new("unsupported 7z coder chain"));
        }
        let flags = h.byte()?;
        if flags & 0xd0 != 0 || flags & 15 == 0 {
            return Err(Error::new("unsupported 7z coder graph"));
        }
        let method = h.take((flags & 15) as usize)?;
        let properties = if flags & 0x20 != 0 {
            let n = h.count(16)?;
            h.take(n)?
        } else {
            &[]
        };
        dictionary(method, properties, limits)?;
    }
    h.expect(12)?;
    let mut unpack = Vec::new();
    let mut total = 0u64;
    for _ in 0..blocks {
        let n = h.number()?;
        total = total
            .checked_add(n)
            .ok_or_else(|| Error::new("7z size overflow"))?;
        if total > limits.total_bytes {
            return Err(Error::new("7z decoded blocks exceed total limit"));
        }
        unpack.push(n);
    }
    let mut crc_defined = 0;
    let mut id = h.byte()?;
    if id == 10 {
        crc_defined = h.defined(blocks)?;
        h.take(crc_defined * 4)?;
        id = h.byte()?;
    }
    if id != 0 {
        return Err(Error::new("invalid 7z unpack info"));
    }
    let mut stream_count = blocks;
    if h.data.get(h.at) == Some(&8) {
        h.byte()?;
        let mut counts = vec![1usize; blocks];
        if h.data.get(h.at) == Some(&13) {
            h.byte()?;
            for n in &mut counts {
                *n = h.count(limits.entries)?;
                if *n == 0 {
                    return Err(Error::new("empty 7z substream groups unsupported"));
                }
            }
        }
        let count: usize = counts.iter().sum();
        stream_count = count;
        if count > limits.entries {
            return Err(Error::new("7z substream count exceeds limit"));
        }
        if h.data.get(h.at) == Some(&9) {
            h.byte()?;
            for (&n, &size) in counts.iter().zip(&unpack) {
                let mut sum = 0u64;
                for _ in 1..n {
                    sum = sum
                        .checked_add(h.number()?)
                        .ok_or_else(|| Error::new("7z substream overflow"))?;
                }
                if sum > size {
                    return Err(Error::new("invalid 7z substream sizes"));
                }
            }
        } else if counts.iter().any(|&n| n > 1) {
            return Err(Error::new("missing 7z substream sizes"));
        }
        if h.data.get(h.at) == Some(&10) {
            // With mixed per-block CRC availability, backend accepts a more complex
            // digest bitmap. This initial profile only permits all or none.
            if crc_defined != 0 && crc_defined != blocks {
                return Err(Error::new("mixed 7z block CRCs unsupported"));
            }
            let inherited = if crc_defined == blocks {
                counts.iter().filter(|&&n| n == 1).count()
            } else {
                0
            };
            h.byte()?;
            h.crcs(count - inherited)?;
        }
        h.expect(0)?;
    }
    h.expect(0)?;
    Ok(stream_count) // end StreamsInfo
}

pub(crate) fn seven_z(data: &[u8], limits: Limits) -> Result<()> {
    let start = data
        .get(..32)
        .ok_or_else(|| Error::new("truncated 7z header"))?;
    let offset = u64::from_le_bytes(start[12..20].try_into().unwrap());
    let size = u64::from_le_bytes(start[20..28].try_into().unwrap());
    if size > limits.header_bytes as u64 {
        return Err(Error::new("7z header exceeds limit"));
    }
    let at = offset
        .checked_add(32)
        .and_then(|n| usize::try_from(n).ok())
        .ok_or_else(|| Error::new("7z offset overflow"))?;
    let end = at
        .checked_add(size as usize)
        .ok_or_else(|| Error::new("7z header overflow"))?;
    let header = data
        .get(at..end)
        .ok_or_else(|| Error::new("truncated 7z next header"))?;
    let mut h = Header {
        data: header,
        at: 0,
    };
    if h.byte()? != 1 {
        return Err(Error::new(
            "encoded 7z headers are not yet supported with bounded decoding",
        ));
    }
    if h.data.get(h.at) == Some(&2) {
        h.byte()?;
        loop {
            if h.byte()? == 0 {
                break;
            }
            let n = h.count(limits.header_bytes)?;
            h.take(n)?;
        }
    }
    let streams = if h.data.get(h.at) == Some(&4) {
        h.byte()?;
        streams(&mut h, limits, at)?
    } else {
        0
    };
    h.expect(5)?;
    let files = h.count(limits.entries)?;
    let mut empty = 0;
    let mut properties = std::collections::BTreeSet::new();
    loop {
        let id = h.byte()?;
        if id == 0 {
            break;
        }
        if !properties.insert(id) {
            return Err(Error::new("duplicate 7z file property"));
        }
        let size = h.count(limits.header_bytes)?;
        let mut property = Header {
            data: h.take(size)?,
            at: 0,
        };
        match id {
            14 => {
                empty = property.bits(files)?;
                property.done()?;
            }
            15 => {
                property.bits(empty)?;
                property.done()?;
            }
            16 => {
                if property.bits(empty)? != 0 {
                    return Err(Error::new("7z anti-items are unsupported"));
                }
                property.done()?;
            }
            17 => {
                property.expect(0)?;
                let names = property.take(size - 1)?;
                if !names.ends_with(&[0, 0])
                    || names.len() % 2 != 0
                    || names.chunks_exact(2).filter(|b| *b == [0, 0]).count() != files
                {
                    return Err(Error::new("invalid 7z filename table"));
                }
            }
            18..=21 => {
                let defined = property.defined(files)?;
                property.expect(0)?;
                property.take(defined * if id == 21 { 4 } else { 8 })?;
                property.done()?;
            }
            _ => return Err(Error::new("unsupported 7z file property")),
        }
    }
    if files - empty != streams {
        return Err(Error::new("7z file and stream counts differ"));
    }
    if !properties.contains(&17) {
        return Err(Error::new("missing 7z names"));
    }
    h.expect(0)?;
    h.done()
}
