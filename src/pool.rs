use futures::task::{Context, Poll};
use futures::Future;
use std::cell::RefCell;
use std::collections::HashMap;
use std::pin::Pin;
use tokio_tasque::{AsyncCall, DefaultCpuTaskQueue, TaskQueueExt};
use trackable::error::ErrorKindExt;

use {BuildCoder, ErasureCode, ErrorKind, Fragment, FragmentBuf, Result};

thread_local! {
    static ERASURE_CODERS: RefCell<HashMap<String, Box<ErasureCode>>> =
        RefCell::new(HashMap::new());
}

/// Thread pool for encoding and decoding data by using an [`ErasureCode`] implementation.
///
/// Internally, this uses [`tokio_tasque::DefaultCpuTaskQueue`] for realizing thread pool functionality.
///
/// [`ErasureCode`]: ./trait.ErasureCode.html
/// [`tokio_tasque::DefaultCpuTaskQueue`]: https://docs.rs/tokio_tasque/0.1/tokio_tasque/struct.DefaultCpuTaskQueue.html
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
    pub fn encode<T>(&self, data: T) -> impl Future<Output = Result<Vec<FragmentBuf>>>
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
    pub fn decode<T>(&self, fragments: Vec<T>) -> impl Future<Output = Result<Vec<u8>>>
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
    ) -> impl Future<Output = Result<Vec<u8>>>
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
        for<'a> F: FnOnce(&'a mut ErasureCode) -> Result<T>,
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
    type Output = Result<T>;
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        Pin::new(&mut self.0).poll(cx).map(|result| match result {
            Ok(result) => track!(result),
            Err(e) => track!(Err(ErrorKind::Other.cause(e).into())),
        })
    }
}

#[cfg(test)]
mod tests {
    use futures::executor::block_on;
    use std::num::NonZeroUsize;
    use std::result::Result;
    use trackable::error::{Failed, MainError};

    use super::*;
    use replica::ReplicaCoder;
    use ErrorKind;

    #[test]
    fn pool_works() -> Result<(), MainError> {
        let data_fragments = track_assert_some!(NonZeroUsize::new(4), Failed);
        let parity_fragments = track_assert_some!(NonZeroUsize::new(2), Failed);

        let coder = ErasureCoderPool::new(ReplicaCoder::new(data_fragments, parity_fragments));
        let data = vec![0, 1, 2, 3];
        let encoded = track!(block_on(coder.encode(data.clone())))?;

        assert_eq!(
            Some(&data),
            block_on(coder.decode(encoded[0..].to_vec())).as_ref().ok()
        );
        assert_eq!(
            Some(&data),
            block_on(coder.decode(encoded[1..].to_vec())).as_ref().ok()
        );
        assert_eq!(
            Some(&data),
            block_on(coder.decode(encoded[2..].to_vec())).as_ref().ok()
        );
        assert_eq!(
            Err(ErrorKind::InvalidInput),
            block_on(coder.decode(encoded[3..].to_vec())).map_err(|e| *e.kind())
        );

        Ok(())
    }
}
