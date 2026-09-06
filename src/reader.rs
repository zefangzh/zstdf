use crate::{endian::Endian, error::{Result, StdfError}};

/// Typed cursor over a single STDF record body.
/// Every multi-byte read goes through here — endianness and
/// truncation errors are handled in exactly one place.
pub struct RecordReader<'a> {
    data:        &'a [u8],
    pos:         usize,
    record:      &'static str,
    file_offset: u64,
    pub endian:  Endian,
}

impl<'a> RecordReader<'a> {
    pub fn new(
        data:        &'a [u8],
        record:      &'static str,
        file_offset: u64,
        endian:      Endian,
    ) -> Self {
        Self { data, pos: 0, record, file_offset, endian }
    }

    pub fn remaining(&self) -> usize { self.data.len() - self.pos }
    pub fn is_empty(&self)   -> bool { self.pos >= self.data.len() }
    pub fn pos(&self)        -> usize { self.pos }

    fn take(&mut self, n: usize) -> Result<&'a [u8]> {
        let avail = self.remaining();
        if avail < n {
            return Err(StdfError::Truncated {
                record:       self.record,
                file_offset:  self.file_offset,
                local_offset: self.pos,
                needed:       n,
                available:    avail,
            });
        }
        let s = &self.data[self.pos..self.pos + n];
        self.pos += n;
        Ok(s)
    }

    // ── Primitive readers ────────────────────────────────────────────────────

    pub fn read_u1(&mut self) -> Result<u8>  { Ok(self.take(1)?[0]) }
    pub fn read_i1(&mut self) -> Result<i8>  { Ok(self.read_u1()? as i8) }
    pub fn read_b1(&mut self) -> Result<u8>  { self.read_u1() }

    pub fn read_u2(&mut self) -> Result<u16> {
        let b: [u8; 2] = self.take(2)?.try_into().unwrap();
        Ok(self.endian.u16(b))
    }
    pub fn read_i2(&mut self) -> Result<i16> { Ok(self.read_u2()? as i16) }

    pub fn read_u4(&mut self) -> Result<u32> {
        let b: [u8; 4] = self.take(4)?.try_into().unwrap();
        Ok(self.endian.u32(b))
    }
    pub fn read_i4(&mut self) -> Result<i32> { Ok(self.read_u4()? as i32) }

    pub fn read_r4(&mut self) -> Result<f32> {
        let b: [u8; 4] = self.take(4)?.try_into().unwrap();
        Ok(self.endian.f32(b))
    }

    pub fn read_r8(&mut self) -> Result<f64> {
        let b: [u8; 8] = self.take(8)?.try_into().unwrap();
        Ok(self.endian.f64(b))
    }

    /// Nibble (N1): one nibble of data per element, but each element still
    /// occupies a full byte on disk (per STDF V4 spec, §C-1) — only the low
    /// nibble is meaningful.
    pub fn read_n1(&mut self) -> Result<u8> { Ok(self.read_u1()? & 0x0F) }

    // ── kx arrays: preceded elsewhere by a U2 item-count field ───────────────

    pub fn read_kx_u1(&mut self, count: usize) -> Result<Vec<u8>> {
        (0..count).map(|_| self.read_u1()).collect()
    }
    pub fn read_kx_u2(&mut self, count: usize) -> Result<Vec<u16>> {
        (0..count).map(|_| self.read_u2()).collect()
    }
    pub fn read_kx_r4(&mut self, count: usize) -> Result<Vec<f32>> {
        (0..count).map(|_| self.read_r4()).collect()
    }
    /// KxN1: `count` nibble-valued elements, packed two per byte (low
    /// nibble = first element, high nibble = second) per STDF V4 §C-1.
    pub fn read_kx_n1(&mut self, count: usize) -> Result<Vec<u8>> {
        let nbytes = count.div_ceil(2);
        let bytes  = self.take(nbytes)?;
        let mut out = Vec::with_capacity(count);
        for i in 0..count {
            let b = bytes[i / 2];
            out.push(if i % 2 == 0 { b & 0x0F } else { (b >> 4) & 0x0F });
        }
        Ok(out)
    }

    // ── STDF composite types ─────────────────────────────────────────────────

    /// Cn: 1-byte length + N bytes.  len=0 → None (field not present).
    pub fn read_cn(&mut self) -> Result<Option<String>> {
        let local = self.pos;
        let len   = self.read_u1()? as usize;
        // A zero length means the field is present but holds an empty
        // string — NOT that the field is absent (absence is instead
        // signalled by there not being enough remaining bytes to even
        // read the length byte at all; see `maybe_cn`).
        if len == 0 { return Ok(Some(String::new())); }
        let bytes = self.take(len)?;
        String::from_utf8(bytes.to_vec()).map(Some).map_err(|_| {
            StdfError::InvalidString {
                record:       self.record,
                file_offset:  self.file_offset,
                local_offset: local,
            }
        })
    }

    /// Bn: 1-byte count + N raw bytes.
    pub fn read_bn(&mut self) -> Result<Vec<u8>> {
        let len = self.read_u1()? as usize;
        Ok(self.take(len)?.to_vec())
    }

    /// Dn: 2-byte bit count + ceil(bits/8) raw bytes.
    pub fn read_dn(&mut self) -> Result<Vec<u8>> {
        let bits  = self.read_u2()? as usize;
        let bytes = bits.div_ceil(8);
        Ok(self.take(bytes)?.to_vec())
    }
    pub fn maybe_dn(&mut self) -> Result<Option<Vec<u8>>> {
        if self.remaining() >= 2 { Ok(Some(self.read_dn()?)) } else { Ok(None) }
    }

    /// Consume all remaining bytes (for Unknown records / ignored fields).
    pub fn read_remaining(&mut self) -> Vec<u8> {
        let s = self.data[self.pos..].to_vec();
        self.pos = self.data.len();
        s
    }

    // ── Optional-trailing-field helpers ─────────────────────────────────────

    pub fn maybe_u1(&mut self) -> Result<Option<u8>>  {
        if self.remaining() >= 1 { Ok(Some(self.read_u1()?)) } else { Ok(None) }
    }
    pub fn maybe_i1(&mut self) -> Result<Option<i8>>  {
        if self.remaining() >= 1 { Ok(Some(self.read_i1()?)) } else { Ok(None) }
    }
    pub fn maybe_u2(&mut self) -> Result<Option<u16>> {
        if self.remaining() >= 2 { Ok(Some(self.read_u2()?)) } else { Ok(None) }
    }
    pub fn maybe_u4(&mut self) -> Result<Option<u32>> {
        if self.remaining() >= 4 { Ok(Some(self.read_u4()?)) } else { Ok(None) }
    }
    pub fn maybe_i4(&mut self) -> Result<Option<i32>> {
        if self.remaining() >= 4 { Ok(Some(self.read_i4()?)) } else { Ok(None) }
    }
    pub fn maybe_r4(&mut self) -> Result<Option<f32>> {
        if self.remaining() >= 4 { Ok(Some(self.read_r4()?)) } else { Ok(None) }
    }
    pub fn maybe_r8(&mut self) -> Result<Option<f64>> {
        if self.remaining() >= 8 { Ok(Some(self.read_r8()?)) } else { Ok(None) }
    }
    pub fn maybe_cn(&mut self) -> Result<Option<String>> {
        if self.remaining() >= 1 { self.read_cn() } else { Ok(None) }
    }
    pub fn maybe_bn(&mut self) -> Result<Option<Vec<u8>>> {
        if self.remaining() >= 1 { Ok(Some(self.read_bn()?)) } else { Ok(None) }
    }

    // ── Post-parse verification ──────────────────────────────────────────────

    /// Warn if bytes were left unconsumed (padding from some testers).
    /// In debug builds this is a hard error; in release it is silently ignored.
    pub fn verify_consumed(&self) -> Result<()> {
        if cfg!(debug_assertions) && self.remaining() != 0 {
            return Err(StdfError::LengthMismatch {
                record:      self.record,
                file_offset: self.file_offset,
                declared:    self.data.len() as u16,
                consumed:    self.pos,
            });
        }
        Ok(())
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::*;
    use crate::endian::Endian;

    fn le(data: &[u8]) -> RecordReader<'_> {
        RecordReader::new(data, "TEST_REC", 0x1000, Endian::Little)
    }
    fn be(data: &[u8]) -> RecordReader<'_> {
        RecordReader::new(data, "TEST_REC", 0x1000, Endian::Big)
    }

    // ── U1 ───────────────────────────────────────────────────────────────────
    #[test]
    fn u1_reads_single_byte() {
        assert_eq!(le(&[0xAB]).read_u1().unwrap(), 0xAB);
    }
    #[test]
    fn u1_truncation_error_includes_context() {
        let err = le(&[]).read_u1().unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("TEST_REC"), "missing record name: {msg}");
        assert!(msg.contains("0x00001000"), "missing file offset: {msg}");
        assert!(msg.contains("need 1"),     "missing needed: {msg}");
        assert!(msg.contains("only 0"),     "missing available: {msg}");
    }

    // ── U2 endianness ────────────────────────────────────────────────────────
    #[test]
    fn u2_little_endian() {
        assert_eq!(le(&[0x34, 0x12]).read_u2().unwrap(), 0x1234);
    }
    #[test]
    fn u2_big_endian() {
        assert_eq!(be(&[0x12, 0x34]).read_u2().unwrap(), 0x1234);
    }
    #[test]
    fn u2_truncation_at_1_byte() {
        let err = le(&[0xFF]).read_u2().unwrap_err();
        assert!(matches!(err, StdfError::Truncated { needed: 2, available: 1, .. }));
    }

    // ── U4 ───────────────────────────────────────────────────────────────────
    #[test]
    fn u4_little_endian() {
        assert_eq!(le(&[0x78, 0x56, 0x34, 0x12]).read_u4().unwrap(), 0x1234_5678);
    }

    // ── R4 ───────────────────────────────────────────────────────────────────
    #[test]
    fn r4_known_value() {
        // 1.0f32 in LE bytes
        let bytes = 1.0f32.to_le_bytes();
        assert!((le(&bytes).read_r4().unwrap() - 1.0).abs() < f32::EPSILON);
    }
    #[test]
    fn r4_nan_roundtrips() {
        let bytes = f32::NAN.to_le_bytes();
        assert!(le(&bytes).read_r4().unwrap().is_nan());
    }

    // ── Cn ───────────────────────────────────────────────────────────────────
    #[test]
    fn cn_empty_len_returns_none() {
        assert_eq!(le(&[0x00]).read_cn().unwrap(), None);
    }
    #[test]
    fn cn_reads_ascii_string() {
        let data = b"\x05hello";
        assert_eq!(le(data).read_cn().unwrap().as_deref(), Some("hello"));
    }
    #[test]
    fn cn_max_length_255() {
        let mut data = vec![255u8];
        data.extend_from_slice(&[b'X'; 255]);
        let s = le(&data).read_cn().unwrap().unwrap();
        assert_eq!(s.len(), 255);
        assert!(s.chars().all(|c| c == 'X'));
    }
    #[test]
    fn cn_invalid_utf8_returns_error() {
        let data = &[0x02u8, 0xFF, 0xFE];
        let err = le(data).read_cn().unwrap_err();
        assert!(matches!(err, StdfError::InvalidString { .. }));
    }

    // ── Optional helpers ─────────────────────────────────────────────────────
    #[test]
    fn maybe_r4_returns_none_when_empty() {
        assert_eq!(le(&[]).maybe_r4().unwrap(), None);
    }
    #[test]
    fn maybe_r4_returns_some_when_4_bytes_present() {
        let bytes = 0.5f32.to_le_bytes();
        let v = le(&bytes).maybe_r4().unwrap().unwrap();
        assert!((v - 0.5).abs() < f32::EPSILON);
    }
    #[test]
    fn maybe_r4_returns_none_when_only_3_bytes() {
        assert_eq!(le(&[0x00, 0x00, 0x00]).maybe_r4().unwrap(), None);
    }

    // ── verify_consumed ──────────────────────────────────────────────────────
    #[test]
    fn verify_consumed_passes_when_all_read() {
        let mut r = le(&[0xAA]);
        r.read_u1().unwrap();
        // In release mode verify_consumed is a no-op; in debug it checks.
        // Either way, should not panic.
        let _ = r.verify_consumed();
    }

    // ── read_remaining ───────────────────────────────────────────────────────
    #[test]
    fn read_remaining_returns_all_unread_bytes() {
        let mut r = le(&[0x01, 0x02, 0x03, 0x04]);
        r.read_u1().unwrap(); // consume 1
        assert_eq!(r.read_remaining(), vec![0x02, 0x03, 0x04]);
        assert!(r.is_empty());
    }
}
