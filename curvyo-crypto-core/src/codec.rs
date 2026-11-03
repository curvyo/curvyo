//! Little-endian reading and writing of the crate's hand-written binary formats.

use crate::CryptoError;

/// Appends fixed-width little-endian fields to a byte vector.
#[derive(Default)]
pub(crate) struct Writer {
    pub(crate) bytes: Vec<u8>,
}

impl Writer {
    pub(crate) fn u8(&mut self, value: u8) {
        self.bytes.push(value);
    }

    pub(crate) fn u16(&mut self, value: u16) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    pub(crate) fn u32(&mut self, value: u32) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    pub(crate) fn u64(&mut self, value: u64) {
        self.bytes.extend_from_slice(&value.to_le_bytes());
    }

    pub(crate) fn raw(&mut self, value: &[u8]) {
        self.bytes.extend_from_slice(value);
    }

    /// Writes a list count; lists longer than `u16::MAX` are refused.
    pub(crate) fn count(&mut self, len: usize) -> Result<(), CryptoError> {
        let len = u16::try_from(len).map_err(|_| CryptoError::Malformed)?;
        self.u16(len);
        Ok(())
    }
}

/// Reads fixed-width little-endian fields; every read past the end is `Malformed`.
pub(crate) struct Reader<'a> {
    bytes: &'a [u8],
}

impl<'a> Reader<'a> {
    pub(crate) fn new(bytes: &'a [u8]) -> Self {
        Self { bytes }
    }

    pub(crate) fn take(&mut self, len: usize) -> Result<&'a [u8], CryptoError> {
        if self.bytes.len() < len {
            return Err(CryptoError::Malformed);
        }
        let (head, tail) = self.bytes.split_at(len);
        self.bytes = tail;
        Ok(head)
    }

    pub(crate) fn array<const N: usize>(&mut self) -> Result<[u8; N], CryptoError> {
        let mut out = [0; N];
        out.copy_from_slice(self.take(N)?);
        Ok(out)
    }

    pub(crate) fn u8(&mut self) -> Result<u8, CryptoError> {
        Ok(self.array::<1>()?[0])
    }

    pub(crate) fn u16(&mut self) -> Result<u16, CryptoError> {
        Ok(u16::from_le_bytes(self.array()?))
    }

    pub(crate) fn u32(&mut self) -> Result<u32, CryptoError> {
        Ok(u32::from_le_bytes(self.array()?))
    }

    pub(crate) fn u64(&mut self) -> Result<u64, CryptoError> {
        Ok(u64::from_le_bytes(self.array()?))
    }

    /// Refuses trailing bytes.
    pub(crate) fn finish(self) -> Result<(), CryptoError> {
        if self.bytes.is_empty() {
            Ok(())
        } else {
            Err(CryptoError::Malformed)
        }
    }
}
