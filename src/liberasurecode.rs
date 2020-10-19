//! An [`ErasureCode`] implementation based on [openstack/liberasurecode].
//!
//! [`ErasureCode`]: ../trait.ErasureCode.html
//! [openstack/liberasurecode]: https://github.com/openstack/liberasurecode
use crate::libec;
use std::num::NonZeroUsize;
use trackable::error::ErrorKindExt;

use crate::{BuildCoder, ErasureCode, Error, ErrorKind, Fragment, FragmentBuf, Result};

pub use crate::libec::{Backend, Checksum};

/// [`LibErasureCoder`] builder.
///
/// [`LibErasureCoder`]: ./struct.LibErasureCoder.html
#[derive(Debug, Clone)]
pub struct LibErasureCoderBuilder {
    data_fragments: NonZeroUsize,
    parity_fragments: NonZeroUsize,
    backend: Backend,
    checksum: Checksum,
}
impl LibErasureCoderBuilder {
    /// Makes a new `LibErasureCoderBuilder` with the default settings.
    pub fn new(data_fragments: NonZeroUsize, parity_fragments: NonZeroUsize) -> Self {
        LibErasureCoderBuilder {
            data_fragments,
            parity_fragments,
            backend: Backend::default(),
            checksum: Checksum::default(),
        }
    }

    /// Sets the type of the erasure coding backend used by the resulting instance.
    ///
    /// The default value is `Backend::default()`.
    pub fn backend(mut self, backend: Backend) -> Self {
        self.backend = backend;
        self
    }

    /// Sets the checksum algorithm used by the resulting instance.
    ///
    /// The default value is `Checksum::default()`.
    pub fn checksum(mut self, checksum: Checksum) -> Self {
        self.checksum = checksum;
        self
    }
}
impl BuildCoder for LibErasureCoderBuilder {
    type Coder = LibErasureCoder;
    fn build_coder(&self) -> Result<Self::Coder> {
        track!(
            libec::Builder::new(self.data_fragments, self.parity_fragments)
                .backend(self.backend)
                .checksum(self.checksum)
                .finish()
                .map(LibErasureCoder::from)
                .map_err(Error::from)
        )
    }
    fn coder_id(&self) -> String {
        format!(
            "liberasurecode:{:?}:{:?}:{}:{}",
            self.backend, self.checksum, self.data_fragments, self.parity_fragments
        )
    }
}

/// An [`ErasureCode`] implementation based on [openstack/liberasurecode].
///
/// [`ErasureCode`]: ../trait.ErasureCode.html
/// [openstack/liberasurecode]: https://github.com/openstack/liberasurecode
///
/// # Examples
///
/// ```
/// use ecpool::{ErasureCode, ErrorKind};
/// use ecpool::liberasurecode::LibErasureCoder;
/// use std::num::NonZeroUsize;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let data_fragments = NonZeroUsize::new(4).ok_or("invalid input")?;
/// let parity_fragments = NonZeroUsize::new(2).ok_or("invalid input")?;
/// let mut coder = LibErasureCoder::new(data_fragments, parity_fragments)?;
///
/// // Encodes
/// let data = vec![0, 1, 2, 3];
/// let encoded = coder.encode(&data)?;
/// let encoded = encoded.iter().map(|f| f.as_ref()).collect::<Vec<_>>();
///
/// // Decodes
/// assert_eq!(Some(&data), coder.decode(&encoded[0..]).as_ref().ok());
/// assert_eq!(Some(&data), coder.decode(&encoded[1..]).as_ref().ok());
/// assert_eq!(Some(&data), coder.decode(&encoded[2..]).as_ref().ok());
/// assert_eq!(Err(ErrorKind::InvalidInput), coder.decode(&encoded[3..]).map_err(|e| *e.kind()));
/// # Ok(())
/// # }
/// ```
pub struct LibErasureCoder {
    inner: libec::ErasureCoder,
}
impl LibErasureCoder {
    /// Makes a new `LibErasureCoder` instance with the default settings.
    ///
    /// This is equivalent to `LibErasureCoderBuilder::new(data_fragments, parity_fragments).build_coder()`.
    #[allow(clippy::new_ret_no_self)]
    pub fn new(data_fragments: NonZeroUsize, parity_fragments: NonZeroUsize) -> Result<Self> {
        track!(LibErasureCoderBuilder::new(data_fragments, parity_fragments).build_coder())
    }

    /// Returns a reference to the inner coder of the instance.
    pub fn inner_ref(&self) -> &libec::ErasureCoder {
        &self.inner
    }

    /// Returns a mutable reference to the inner coder of the instance.
    pub fn inner_mut(&mut self) -> &mut libec::ErasureCoder {
        &mut self.inner
    }

    /// Takes the owership of the instance and returns the inner coder.
    pub fn into_inner(self) -> libec::ErasureCoder {
        self.inner
    }
}
impl ErasureCode for LibErasureCoder {
    fn data_fragments(&self) -> NonZeroUsize {
        self.inner.data_fragments()
    }

    fn parity_fragments(&self) -> NonZeroUsize {
        self.inner.parity_fragments()
    }

    fn encode(&mut self, data: &[u8]) -> Result<Vec<FragmentBuf>> {
        let fragments = self.inner.encode(data)?;
        Ok(fragments)
    }

    fn decode(&mut self, fragments: &[&Fragment]) -> Result<Vec<u8>> {
        let data = self.inner.decode(fragments)?;
        Ok(data)
    }

    fn reconstruct(&mut self, index: usize, fragments: &[&Fragment]) -> Result<Vec<u8>> {
        let fragment = self.inner.reconstruct(index, fragments.iter())?;
        Ok(fragment)
    }
}
impl From<libec::ErasureCoder> for LibErasureCoder {
    fn from(f: libec::ErasureCoder) -> Self {
        LibErasureCoder { inner: f }
    }
}

impl From<libec::Error> for Error {
    fn from(f: libec::Error) -> Self {
        use crate::libec::Error::*;
        match f {
            InsufficientFragments => ErrorKind::InvalidInput.cause(f).into(),
            BadChecksum | BadHeader => ErrorKind::CorruptedFragments.cause(f).into(),
            _ => ErrorKind::Other.cause(f).into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroUsize;

    use super::*;
    use crate::{ErasureCode, ErrorKind};

    #[test]
    fn it_works() {
        let data_fragments = NonZeroUsize::new(4).unwrap();
        let parity_fragments = NonZeroUsize::new(2).unwrap();
        let mut coder = LibErasureCoder::new(data_fragments, parity_fragments).unwrap();
        let data = vec![0, 1, 2, 3];
        let encoded = coder.encode(&data).unwrap();
        let encoded = encoded.iter().map(|f| f.as_ref()).collect::<Vec<_>>();

        assert_eq!(Some(&data), coder.decode(&encoded[0..]).as_ref().ok());
        assert_eq!(Some(&data), coder.decode(&encoded[1..]).as_ref().ok());
        assert_eq!(Some(&data), coder.decode(&encoded[2..]).as_ref().ok());
        assert_eq!(
            Err(ErrorKind::InvalidInput),
            coder.decode(&encoded[3..]).map_err(|e| *e.kind())
        );
    }
}
