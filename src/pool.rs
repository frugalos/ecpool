use fibers_tasque::{AsyncCall, DefaultCpuTaskQueue, TaskQueueExt};
use futures::{Async, Future, Poll};
use std::cell::RefCell;
use std::collections::HashMap;
use trackable::error::ErrorKindExt;

use crate::{BuildCoder, ErasureCode, Error, ErrorKind, Fragment, FragmentBuf, Result};

thread_local! {
    static ERASURE_CODERS: RefCell<HashMap<String, Box<dyn ErasureCode>>> =
        RefCell::new(HashMap::new());
}

/// Thread pool for encoding and decoding data by using an [`ErasureCode`] implementation.
///
/// Internally, this uses [`fibers_tasque::DefaultCpuTaskQueue`] for realizing thread pool functionality.
///
/// [`ErasureCode`]: ./trait.ErasureCode.html
/// [`fibers_tasque::DefaultCpuTaskQueue`]: https://docs.rs/fibers_tasque/0.1/fibers_tasque/struct.DefaultCpuTaskQueue.html
#[derive(Debug, Clone)]
pub struct ErasureCoderPool<B> {
    builder: B,
}
impl<B: BuildCoder> ErasureCoderPool<B> {
    /// Makes a new `ErasureCoderPool` instance.
    pub fn new(builder: B) -> Self {
        ErasureCoderPool { builder }
    }

    /// Encodes the given data to fragments asynchronously.
    ///
    /// The encoding process will be executed on a thread in the pool.
    ///
    /// The result vector contains `N` data fragments and `M` parity fragments
    /// (where `N = self.data_fragments()` and `M = self.parity_fragments()`).
    pub fn encode<T>(&self, data: T) -> impl Future<Item = Vec<FragmentBuf>, Error = Error>
    where
        T: AsRef<[u8]> + Send + 'static,
    {
        let builder = self.builder.clone();
        let result = DefaultCpuTaskQueue
            .async_call(move || Self::with_coder(&builder, |coder| coder.encode(data.as_ref())));
        LazyResult(result)
    }

    /// Decodes the original data from the given fragments asynchronously.
    ///
    /// The decoding process will be executed on a thread in the pool.
    ///
    /// Note whether the correctness of the result data has been validated depends on the implementations.
    pub fn decode<T>(&self, fragments: Vec<T>) -> impl Future<Item = Vec<u8>, Error = Error>
    where
        T: AsRef<Fragment> + Send + 'static,
    {
        let builder = self.builder.clone();
        let result = DefaultCpuTaskQueue.async_call(move || {
            let fragments = fragments.iter().map(|f| f.as_ref()).collect::<Vec<_>>();
            Self::with_coder(&builder, |coder| coder.decode(&fragments))
        });
        LazyResult(result)
    }

    /// Reconstructs the fragment specified by the given index from other fragments asynchronously.
    ///
    /// The reconstruction process will be executed on a thread in the pool.
    pub fn reconstruct<T>(
        &self,
        index: usize,
        fragments: Vec<T>,
    ) -> impl Future<Item = Vec<u8>, Error = Error>
    where
        T: AsRef<Fragment> + Send + 'static,
    {
        let builder = self.builder.clone();
        let result = DefaultCpuTaskQueue.async_call(move || {
            let fragments = fragments.iter().map(|f| f.as_ref()).collect::<Vec<_>>();
            Self::with_coder(&builder, |coder| coder.reconstruct(index, &fragments))
        });
        LazyResult(result)
    }

    fn with_coder<F, T>(builder: &B, f: F) -> Result<T>
    where
        for<'a> F: FnOnce(&'a mut dyn ErasureCode) -> Result<T>,
    {
        ERASURE_CODERS.with(|coders| {
            let coder_id = builder.coder_id();
            let mut coders = coders.borrow_mut();
            if !coders.contains_key(&coder_id) {
                let coder = builder.build_coder()?;
                coders.insert(coder_id.clone(), Box::new(coder));
            }
            f(coders.get_mut(&coder_id).expect("Never fails").as_mut())
        })
    }
}

struct LazyResult<T>(AsyncCall<Result<T>>);
impl<T> Future for LazyResult<T> {
    type Item = T;
    type Error = Error;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        if let Async::Ready(result) = track!(self.0.poll().map_err(|e| ErrorKind::Other.cause(e)))?
        {
            let value = result?;
            Ok(Async::Ready(value))
        } else {
            Ok(Async::NotReady)
        }
    }
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroUsize;
    use std::result::Result;
    use trackable::error::{Failed, MainError};

    use super::*;
    use crate::replica::ReplicaCoder;
    use crate::ErrorKind;

    #[test]
    fn pool_works() -> Result<(), MainError> {
        let data_fragments = track_assert_some!(NonZeroUsize::new(4), Failed);
        let parity_fragments = track_assert_some!(NonZeroUsize::new(2), Failed);

        let coder = ErasureCoderPool::new(ReplicaCoder::new(data_fragments, parity_fragments));
        let data = vec![0, 1, 2, 3];
        let encoded = track!(fibers_global::execute(coder.encode(data.clone())))?;

        assert_eq!(
            Some(&data),
            fibers_global::execute(coder.decode(encoded[0..].to_vec()))
                .as_ref()
                .ok()
        );
        assert_eq!(
            Some(&data),
            fibers_global::execute(coder.decode(encoded[1..].to_vec()))
                .as_ref()
                .ok()
        );
        assert_eq!(
            Some(&data),
            fibers_global::execute(coder.decode(encoded[2..].to_vec()))
                .as_ref()
                .ok()
        );
        assert_eq!(
            Err(ErrorKind::InvalidInput),
            fibers_global::execute(coder.decode(encoded[3..].to_vec())).map_err(|e| *e.kind())
        );

        Ok(())
    }
}
