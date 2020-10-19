//! An [`ErasureCode`] implementation that simply replicates the input data.
//!
//! [`ErasureCode`]: ../trait.ErasureCode.html
use std::num::NonZeroUsize;

use crate::{BuildCoder, ErasureCode, ErrorKind, Fragment, FragmentBuf, Result};

/// An [`ErasureCode`] implementation that simply replicates the input data.
///
/// Note that this is provided for example and testing purposes only and not intended to use in production.
///
/// [`ErasureCode`]: ../trait.ErasureCode.html
///
/// # Examples
///
/// ```
/// use ecpool::{ErasureCode, ErrorKind};
/// use ecpool::replica::ReplicaCoder;
/// use std::num::NonZeroUsize;
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let data_fragments = NonZeroUsize::new(4).ok_or("invalid input")?;
/// let parity_fragments = NonZeroUsize::new(2).ok_or("invalid input")?;
/// let mut coder = ReplicaCoder::new(data_fragments, parity_fragments);
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
#[derive(Debug, Clone)]
pub struct ReplicaCoder {
    data_fragments: NonZeroUsize,
    parity_fragments: NonZeroUsize,
}
impl ReplicaCoder {
    /// Makes a new `ReplicaCoder` instance.
    pub fn new(data_fragments: NonZeroUsize, parity_fragments: NonZeroUsize) -> Self {
        ReplicaCoder {
            data_fragments,
            parity_fragments,
        }
    }
}
impl ErasureCode for ReplicaCoder {
    fn data_fragments(&self) -> NonZeroUsize {
        self.data_fragments
    }

    fn parity_fragments(&self) -> NonZeroUsize {
        self.parity_fragments
    }

    fn encode(&mut self, data: &[u8]) -> Result<Vec<FragmentBuf>> {
        let mut fragments = Vec::with_capacity(self.fragments().get());
        fragments.push(Vec::from(data));
        for _ in 1..self.data_fragments.get() {
            fragments.push(Vec::new());
        }
        for _ in 0..self.parity_fragments.get() {
            fragments.push(Vec::from(data));
        }
        Ok(fragments)
    }

    fn decode(&mut self, fragments: &[&Fragment]) -> Result<Vec<u8>> {
        track_assert!(
            fragments.len() >= self.data_fragments.get(),
            ErrorKind::InvalidInput,
            "fragments={}, data_fragments={}",
            fragments.len(),
            self.data_fragments
        );
        let data = track_assert_some!(
            fragments.iter().find(|f| !f.is_empty()),
            ErrorKind::CorruptedFragments,
            "No replica fragment is found"
        );
        Ok(data.to_vec())
    }
}
impl BuildCoder for ReplicaCoder {
    type Coder = Self;

    fn build_coder(&self) -> Result<Self::Coder> {
        Ok(self.clone())
    }

    fn coder_id(&self) -> String {
        format!("replica:{}:{}", self.data_fragments, self.parity_fragments)
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

        let mut coder = ReplicaCoder::new(data_fragments, parity_fragments);
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
