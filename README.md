ecpool
======

[![Crates.io: ecpool](https://img.shields.io/crates/v/ecpool.svg)](https://crates.io/crates/ecpool)
[![Documentation](https://docs.rs/ecpool/badge.svg)](https://docs.rs/ecpool)
[![Build Status](https://travis-ci.org/frugalos/ecpool.svg?branch=master)](https://travis-ci.org/frugalos/ecpool)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

This crate provides thread pool ([`ErasureCoderPool`]) for managing executions of erasure coding.

[Documentation](https://docs.rs/ecpool)

`ecpool` also provides [`ErasureCode`] trait defines erasure coding interface
and of which implemtations can be executed via [`ErasureCoderPool`].

There are some built-in implementations of the trait:
- [`LibErasureCoder`]:
  - This implementation uses [`liberasurecode`] crate that is a wrapper for [openstack/liberasurecode] library.
  - It is highly optimized and stable but only available in Unix environments.
- [`ReplicaCoder`]:
  - This implementation simply replicates the input data.
  - It is provided for example and testing purposes only and not intended to use in production.


Build Prerequisites
-------------------

It is required to install [openstack/liberasurecode] and its dependencies by executing
the following commands before building this crate:

```console
$ git clone https://github.com/frugalos/liberasurecode
$ cd liberasurecode && sudo ./install_deps.sh
```

Examples
--------

Basic usage:
```rust
use ecpool::replica::ReplicaCoder;
use ecpool::{ErrorKind, ErasureCoderPool};
use futures::executor::block_on;
use std::num::NonZeroUsize;
use std::result::Result;
use trackable::error::{Failure, Failed};

// Creates a pool
let data_fragments = NonZeroUsize::new(4).ok_or_else(|| Failure::from(Failed))?;
let parity_fragments = NonZeroUsize::new(2).ok_or_else(|| Failure::from(Failed))?;
let coder = ErasureCoderPool::new(ReplicaCoder::new(data_fragments, parity_fragments));

// Encodes
let data = vec![0, 1, 2, 3];
let encoded = block_on(coder.encode(data.clone()))?;

// Decodes
assert_eq!(
    Some(&data),
    block_on(coder.decode(encoded[0..].to_vec()))
        .as_ref()
        .ok()
);
assert_eq!(
    Some(&data),
    block_on(coder.decode(encoded[1..].to_vec()))
        .as_ref()
        .ok()
);
assert_eq!(
    Some(&data),
    block_on(coder.decode(encoded[2..].to_vec()))
        .as_ref()
        .ok()
);
assert_eq!(
    Err(ErrorKind::InvalidInput),
    block_on(coder.decode(encoded[3..].to_vec())).map_err(|e| *e.kind())
);
```

[`ErasureCoderPool`]: https://docs.rs/ecpool/0.1/struct.ErasureCoderPool.html
[`ErasureCode`]: https://docs.rs/ecpool/0.1/trait.ErasureCode.html
[`liberasurecode`]: https://github.com/frugalos/liberasurecode
[openstack/liberasurecode]: https://github.com/openstack/liberasurecode
[`LibErasureCoder`]: https://docs.rs/ecpool/0.1/liberasurecode/struct.LibErasureCoder.html
[`ReplicaCoder`]: https://docs.rs/ecpool/0.1/replica/struct.ReplicaCoder.html
