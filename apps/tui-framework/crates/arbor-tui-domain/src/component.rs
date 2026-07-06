use crate::signal::SignalDep;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Default)]
pub struct PropsRevision(u64);

impl PropsRevision {
    pub const ZERO: Self = Self(0);

    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    pub const fn get(self) -> u64 {
        self.0
    }
}

pub trait ComponentProps: 'static {
    fn revision(&self) -> PropsRevision {
        PropsRevision::ZERO
    }

    fn signal_deps(&self) -> &[SignalDep] {
        &[]
    }
}

#[derive(Clone, Debug)]
pub struct PropsRevisionBuilder {
    hash: u64,
}

impl Default for PropsRevisionBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl PropsRevisionBuilder {
    const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01b3;

    pub const fn new() -> Self {
        Self { hash: Self::OFFSET }
    }

    pub fn field_tag(&mut self, tag: u8) -> &mut Self {
        self.write_u8(tag)
    }

    pub fn write_bool(&mut self, value: bool) -> &mut Self {
        self.write_u8(u8::from(value))
    }

    pub fn write_u8(&mut self, value: u8) -> &mut Self {
        self.write_bytes(&[value])
    }

    pub fn write_u16(&mut self, value: u16) -> &mut Self {
        self.write_bytes(&value.to_le_bytes())
    }

    pub fn write_u32(&mut self, value: u32) -> &mut Self {
        self.write_bytes(&value.to_le_bytes())
    }

    pub fn write_u64(&mut self, value: u64) -> &mut Self {
        self.write_bytes(&value.to_le_bytes())
    }

    pub fn write_usize(&mut self, value: usize) -> &mut Self {
        self.write_u64(value as u64)
    }

    pub fn write_f32(&mut self, value: f32) -> &mut Self {
        self.write_u32(value.to_bits())
    }

    pub fn write_str(&mut self, value: &str) -> &mut Self {
        self.write_usize(value.len());
        self.write_bytes(value.as_bytes())
    }

    pub fn write_option_u16(&mut self, value: Option<u16>) -> &mut Self {
        match value {
            Some(value) => {
                self.write_bool(true);
                self.write_u16(value)
            }
            None => self.write_bool(false),
        }
    }

    pub fn write_bytes(&mut self, bytes: &[u8]) -> &mut Self {
        for byte in bytes {
            self.hash ^= u64::from(*byte);
            self.hash = self.hash.wrapping_mul(Self::PRIME);
        }
        self
    }

    pub const fn finish(&self) -> PropsRevision {
        PropsRevision(self.hash)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn revision_builder_is_stable_for_same_inputs() {
        let left = PropsRevisionBuilder::new()
            .field_tag(1)
            .write_str("hello")
            .field_tag(2)
            .write_u16(7)
            .finish();
        let right = PropsRevisionBuilder::new()
            .field_tag(1)
            .write_str("hello")
            .field_tag(2)
            .write_u16(7)
            .finish();

        assert_eq!(left, right);
    }

    #[test]
    fn revision_builder_changes_when_inputs_change() {
        let left = PropsRevisionBuilder::new().write_str("hello").finish();
        let right = PropsRevisionBuilder::new().write_str("world").finish();

        assert_ne!(left, right);
    }
}
