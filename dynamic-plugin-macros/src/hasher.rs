use std::hash::Hasher;

#[derive(Default)]
pub struct PluginSignatureHasher {
    inner: u64,
    #[cfg(feature = "debug-hashes")]
    debug: String,
}

#[cfg(feature = "debug-hashes")]
impl std::fmt::Debug for PluginSignatureHasher {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.debug)
    }
}

impl Hasher for PluginSignatureHasher {
    fn finish(&self) -> u64 {
        self.inner
    }

    fn write(&mut self, bytes: &[u8]) {
        #[cfg(feature = "debug-hashes")]
        self.debug.push_str(&String::from_utf8_lossy(bytes));
        for byte in bytes {
            self.inner = self.inner.wrapping_add(u64::from(*byte)).rotate_left(8);
        }
    }
}
