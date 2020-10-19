//! This crate provides thread pool([`ErasureCoderPool`]) for managing executions of erasure coding.
//!
//! `ecpool` also provides [`ErasureCode`] trait defines erasure coding interface
//! and of which implemtations can be executed via [`ErasureCoderPool`].
//!
//! There are some built-in implementations of the trait:
//! - [`LibErasureCoder`]:
//!   - This implementation uses [`liberasurecode`] crate that is a wrapper for [openstack/liberasurecode] library.
//!   - It is highly optimized and stable but only available in Unix environments.
//! - [`ReplicaCoder`]:
//!   - This implementation simply replicates the input data.
//!   - It is provided for example and testing purposes only and not intended to use in production.
//!
//!
//! # Build Prerequisites
//!
//! It is required to install [openstack/liberasurecode] and its dependencies by executing
//! the following commands before building this crate:
//!
//! ```console
//! $ git clone https://github.com/frugalos/liberasurecode
//! $ cd liberasurecode && sudo ./install_deps.sh
//! ```
//!
//! # Examples
//!
//! Basic usage:
//! ```
//! # extern crate ecpool;
//! # extern crate fibers_global;
//! # extern crate trackable;
//! use ecpool::replica::ReplicaCoder;
//! use ecpool::{ErrorKind, ErasureCoderPool};
//! use std::num::NonZeroUsize;
//! use std::result::Result;
//! use trackable::error::{Failure, Failed};
//!
//! # fn main() -> Result<(), trackable::error::MainError> {
//! // Creates a pool
//! let data_fragments = NonZeroUsize::new(4).ok_or_else(|| Failure::from(Failed))?;
//! let parity_fragments = NonZeroUsize::new(2).ok_or_else(|| Failure::from(Failed))?;
//! let coder = ErasureCoderPool::new(ReplicaCoder::new(data_fragments, parity_fragments));
//!
//! // Encodes
//! let data = vec![0, 1, 2, 3];
//! let encoded = fibers_global::execute(coder.encode(data.clone()))?;
//!
//! // Decodes
//! assert_eq!(
//!     Some(&data),
//!     fibers_global::execute(coder.decode(encoded[0..].to_vec()))
//!         .as_ref()
//!         .ok()
//! );
//! assert_eq!(
//!     Some(&data),
//!     fibers_global::execute(coder.decode(encoded[1..].to_vec()))
//!         .as_ref()
//!         .ok()
//! );
//! assert_eq!(
//!     Some(&data),
//!     fibers_global::execute(coder.decode(encoded[2..].to_vec()))
//!         .as_ref()
//!         .ok()
//! );
//! assert_eq!(
//!     Err(ErrorKind::InvalidInput),
//!     fibers_global::execute(coder.decode(encoded[3..].to_vec())).map_err(|e| *e.kind())
//! );
//! # Ok(())
//! # }
//! ```
//!
//! [`ErasureCoderPool`]: ./struct.ErasureCoderPool.html
//! [`ErasureCode`]: ./trait.ErasureCode.html
//! [`liberasurecode`]: https://github.com/frugalos/liberasurecode
//! [openstack/liberasurecode]: https://github.com/openstack/liberasurecode
//! [`LibErasureCoder`]: ./liberasurecode/struct.LibErasureCoder.html
//! [`ReplicaCoder`]: ./replica/struct.ReplicaCoder.html
#![warn(missing_docs)]
extern crate fibers;
#[cfg(test)]
extern crate fibers_global;
extern crate fibers_tasque;
extern crate futures;
#[macro_use]
extern crate trackable;

#[cfg(unix)]
extern crate liberasurecode as libec;

use std::num::NonZeroUsize;

pub use crate::error::{Error, ErrorKind};
pub use crate::pool::ErasureCoderPool;

#[cfg(unix)]
pub mod liberasurecode;
pub mod replica;

mod error;
mod pool;

/// This crate specific [`Result`] type.
///
/// [`Result`]: https://doc.rust-lang.org/std/result/enum.Result.html
pub type Result<T> = std::result::Result<T, Error>;

/// A fragment.
pub type Fragment = [u8];

/// An owned fragment.
pub type FragmentBuf = Vec<u8>;

/// This trait allows for encoding and decoding data by using some erasure coding algorithm.
pub trait ErasureCode {
    /// Returns the number of data fragments that the instance uses when encoding/decoding data.
    fn data_fragments(&self) -> NonZeroUsize;

    /// Returns the number of parity fragments that the instance uses when encoding/decoding data.
    fn parity_fragments(&self) -> NonZeroUsize;

    /// The total number of data fragments and parity fragments of the instance.
    fn fragments(&self) -> NonZeroUsize {
        unsafe {
            NonZeroUsize::new_unchecked(self.data_fragments().get() + self.parity_fragments().get())
        }
    }

    /// Encodes the given data to fragments.
    ///
    /// The result vector contains `N` data fragments and `M` parity fragments
    /// (where `N = self.data_fragments()` and `M = self.parity_fragments()`).
    fn encode(&mut self, data: &[u8]) -> Result<Vec<FragmentBuf>>;

    /// Decodes the original data from the given fragments.
    ///
    /// Note whether the correctness of the result data has been validated depends on the implementations.
    fn decode(&mut self, fragments: &[&Fragment]) -> Result<Vec<u8>>;

    /// Reconstructs the fragment specified by the given index from other fragments.
    fn reconstruct(&mut self, index: usize, fragments: &[&Fragment]) -> Result<Vec<u8>> {
        track_assert!(
            index < self.fragments().get(),
            ErrorKind::Other,
            "Too large index: index={}, fragments={}",
            index,
            self.fragments()
        );
        let decoded = self.decode(fragments)?;
        let mut encoded = self.encode(&decoded)?;
        Ok(encoded.swap_remove(index))
    }
}

/// This trait allows for building instances of an implementaion of [`ErasureCode`] trait.
///
/// [`ErasureCode`]: ./trait.ErasureCode.html
pub trait BuildCoder: Clone + Send + 'static {
    /// The type of `ErasureCode` implementaion to be built.
    type Coder: ErasureCode;

    /// Builds an instance of the `ErasureCode` implementaion.
    fn build_coder(&self) -> Result<Self::Coder>;

    /// Returns the identifier that distinguishes the kind of instances to be built.
    ///
    /// If two coder instances use different parameters for encoding/decoding,
    /// the identifiers that associated to those must be different.
    fn coder_id(&self) -> String;
}
