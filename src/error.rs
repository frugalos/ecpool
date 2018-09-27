use trackable::error::ErrorKind as TrackableErrorKind;
use trackable::error::TrackableError;

/// This crate specific [`Error`] type.
///
/// [`Error`]: https://doc.rust-lang.org/std/error/trait.Error.html
#[derive(Debug, TrackableError)]
pub struct Error(TrackableError<ErrorKind>);

/// Possible error kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorKind {
    /// Failed to decode data due to input fragments corruption.
    CorruptedFragments,

    /// Input is invalid.
    InvalidInput,

    /// Other error.
    Other,
}
impl TrackableErrorKind for ErrorKind {}
