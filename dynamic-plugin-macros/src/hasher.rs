use std::hash::Hasher;

#[derive(Default)]
pub struct PluginSignatureHasher(u64);

impl Hasher for PluginSignatureHasher {
    fn finish(&self) -> u64 {
        self.0
    }

    fn write(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.0 = self.0.wrapping_add(*byte as u64).rotate_left(8);
        }
    }
}
