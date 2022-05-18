#![forbid(unsafe_code, future_incompatible)]
#![forbid(rust_2018_idioms, rust_2018_compatibility)]
#![deny(missing_debug_implementations, bad_style)]
#![deny(missing_docs)]
#![cfg_attr(test, deny(warnings))]

//! ## Examples
//! ```rust
//! use fly_accept_encoding::{Encoding,Error};
//! use http::header::{HeaderMap, HeaderValue, ACCEPT_ENCODING};
//!
//! # fn main () -> Result<(), Error> {
//! let mut headers = HeaderMap::new();
//! headers.insert(ACCEPT_ENCODING, HeaderValue::from_str("gzip, deflate, br").unwrap());
//!
//! let encoding = fly_accept_encoding::parse(&headers)?;
//! assert_eq!(encoding, Some(Encoding::Gzip));
//! # Ok(())}
//! ```
//!
//! ```rust
//! use fly_accept_encoding::{Encoding,Error};
//! use http::header::{HeaderMap, HeaderValue, ACCEPT_ENCODING};
//!
//! # fn main () -> Result<(), Error> {
//! let mut headers = HeaderMap::new();
//! headers.insert(ACCEPT_ENCODING, HeaderValue::from_str("gzip;q=0.5, deflate;q=0.9, br;q=1.0").unwrap());
//!
//! let encoding = fly_accept_encoding::parse(&headers)?;
//! assert_eq!(encoding, Some(Encoding::Brotli));
//! # Ok(())}
//! ```

mod error;

pub use crate::error::{Error, Result};
use http::header::{HeaderMap, HeaderValue, ACCEPT_ENCODING};
use itertools::Itertools;

/// Encodings to use.
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum Encoding {
    /// The Gzip encoding.
    Gzip,
    /// The Deflate encoding.
    Deflate,
    /// The Brotli encoding.
    Brotli,
    /// The Zstd encoding.
    Zstd,
    /// No encoding.
    Identity,
}

impl Encoding {
    /// Parses a given string into its corresponding encoding.
    fn parse(s: &str) -> Result<Option<Encoding>> {
        match s {
            "gzip" => Ok(Some(Encoding::Gzip)),
            "deflate" => Ok(Some(Encoding::Deflate)),
            "br" => Ok(Some(Encoding::Brotli)),
            "zstd" => Ok(Some(Encoding::Zstd)),
            "identity" => Ok(Some(Encoding::Identity)),
            "*" => Ok(None),
            _ => Err(Error::UnknownEncoding),
        }
    }

    /// Converts the encoding into its' corresponding header value.
    pub fn to_header_value(self) -> HeaderValue {
        match self {
            Encoding::Gzip => HeaderValue::from_str("gzip").unwrap(),
            Encoding::Deflate => HeaderValue::from_str("deflate").unwrap(),
            Encoding::Brotli => HeaderValue::from_str("br").unwrap(),
            Encoding::Zstd => HeaderValue::from_str("zstd").unwrap(),
            Encoding::Identity => HeaderValue::from_str("identity").unwrap(),
        }
    }
}

/// Parse a set of HTTP headers into a single option yielding an `Encoding` that the client prefers.
///
/// If you're looking for an easy way to determine the best encoding for the client and support every [`Encoding`] listed, this is likely what you want.
///
/// Note that a result of `None` indicates there preference is expressed on which encoding to use.
/// Either the `Accept-Encoding` header is not present, or `*` is set as the most preferred encoding.
pub fn parse(headers: &HeaderMap) -> Result<Option<Encoding>> {
    preferred(encodings_iter(headers))
}

/// Select the encoding with the largest qval or the first with qval ~= 1
pub fn preferred(
    mut encodings: impl Iterator<Item = Result<(Option<Encoding>, f32)>>,
) -> Result<Option<Encoding>> {
    Ok(encodings
        .fold_ok((None, 0.0), |(ae, aq), (e, q)| {
            if (aq - 1.0f32).abs() < 0.01 {
                (ae, aq)
            } else if q > aq {
                (e, q)
            } else {
                (ae, aq)
            }
        })?
        .0)
}

/// Parse a set of HTTP headers into a vector containing tuples of options containing encodings and their corresponding q-values.
///
/// If you're looking for more fine-grained control over what encoding to choose for the client, or if you don't support every [`Encoding`] listed, this is likely what you want.
///
/// Note that a result of `None` indicates there preference is expressed on which encoding to use.
/// Either the `Accept-Encoding` header is not present, or `*` is set as the most preferred encoding.
/// ## Examples
/// ```rust
/// use fly_accept_encoding::{Encoding,Error};
/// use http::header::{HeaderMap, HeaderValue, ACCEPT_ENCODING};
///
/// # fn main () -> Result<(), Error> {
/// let mut headers = HeaderMap::new();
/// headers.insert(ACCEPT_ENCODING, HeaderValue::from_str("zstd;q=1.0, deflate;q=0.8, br;q=0.9").unwrap());
///
/// let encodings = fly_accept_encoding::encodings(&headers)?;
/// for (encoding, qval) in encodings {
///     println!("{:?} {}", encoding, qval);
/// }
/// # Ok(())}
/// ```
pub fn encodings(headers: &HeaderMap) -> Result<Vec<(Option<Encoding>, f32)>> {
    encodings_iter(headers).collect()
}

/// Parse a set of HTTP headers into an iterator containing tuples of options containing encodings and their corresponding q-values.
pub fn encodings_iter(
    headers: &HeaderMap,
) -> impl Iterator<Item = Result<(Option<Encoding>, f32)>> + '_ {
    headers
        .get_all(ACCEPT_ENCODING)
        .iter()
        .map(|hval| hval.to_str().map_err(|_| Error::InvalidEncoding))
        .map_ok(|s| s.split(',').map(str::trim))
        .flatten_ok()
        .filter_map_ok(|v| {
            let mut v = v.splitn(2, ";q=");
            let encoding = Encoding::parse(v.next()?).ok()?; // ignore unknown encodings
            let qval = match v.next().map(|v| v.parse()) {
                Some(Ok(f)) if f > 1.0 => return Some(Err(Error::InvalidEncoding)), // q-values over 1 are unacceptable,
                Some(Ok(f)) => f,
                Some(Err(_)) => return Some(Err(Error::InvalidEncoding)),
                None => 1.0f32,
            };
            Some(Ok((encoding, qval)))
        })
        .map(|r| r?) // flatten Result<Result<...
}
