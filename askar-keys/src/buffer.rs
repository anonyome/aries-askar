use alloc::{
    borrow::Cow,
    string::{String, ToString},
    vec::Vec,
};
use core::{
    cmp::Ordering,
    fmt::{self, Debug, Formatter},
    marker::PhantomData,
    mem::{self, ManuallyDrop},
    ops::{Deref, DerefMut},
};
use fmt::Write;

use aead::{
    generic_array::{typenum, ArrayLength, GenericArray},
    Buffer,
};
use serde::{de::Visitor, Deserialize, Deserializer, Serialize, Serializer};
use zeroize::Zeroize;

use crate::{error::Error, random::random_array};

/// A secure key representation for fixed-length keys
#[derive(Clone, Debug, Hash, Zeroize)]
pub struct ArrayKey<L: ArrayLength<u8>>(GenericArray<u8, L>);

impl<L: ArrayLength<u8>> ArrayKey<L> {
    pub const SIZE: usize = L::USIZE;

    #[inline]
    pub fn from_slice<D: AsRef<[u8]>>(data: D) -> Self {
        Self(GenericArray::clone_from_slice(data.as_ref()))
    }

    #[inline]
    pub fn extract(self) -> GenericArray<u8, L> {
        self.0.clone()
    }

    #[inline]
    pub fn random() -> Self {
        Self(random_array())
    }
}

impl<L: ArrayLength<u8>> Default for ArrayKey<L> {
    #[inline]
    fn default() -> Self {
        Self(GenericArray::default())
    }
}

impl<L: ArrayLength<u8>> From<GenericArray<u8, L>> for ArrayKey<L> {
    fn from(key: GenericArray<u8, L>) -> Self {
        Self(key)
    }
}

impl<L: ArrayLength<u8>> Deref for ArrayKey<L> {
    type Target = GenericArray<u8, L>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<L: ArrayLength<u8>> DerefMut for ArrayKey<L> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<L: ArrayLength<u8>> PartialEq for ArrayKey<L> {
    fn eq(&self, other: &Self) -> bool {
        **self == **other
    }
}
impl<L: ArrayLength<u8>> Eq for ArrayKey<L> {}

impl<L: ArrayLength<u8>> PartialOrd for ArrayKey<L> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.0.partial_cmp(&*other)
    }
}
impl<L: ArrayLength<u8>> Ord for ArrayKey<L> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.cmp(&*other)
    }
}

impl<L: ArrayLength<u8>> Serialize for ArrayKey<L> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // create an array twice the size of L on the stack (could be made clearer with const generics)
        let mut hex_str = GenericArray::<u8, typenum::UInt<L, typenum::B0>>::default();
        hex::encode_to_slice(&self.0.as_slice(), &mut hex_str).unwrap();
        serializer.serialize_str(core::str::from_utf8(&hex_str[..]).unwrap())
    }
}

impl<'a, L: ArrayLength<u8>> Deserialize<'a> for ArrayKey<L> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'a>,
    {
        deserializer.deserialize_str(KeyVisitor { _pd: PhantomData })
    }
}

impl<L: ArrayLength<u8>> Drop for ArrayKey<L> {
    fn drop(&mut self) {
        self.zeroize();
    }
}

struct KeyVisitor<L: ArrayLength<u8>> {
    _pd: PhantomData<L>,
}

impl<'a, L: ArrayLength<u8>> Visitor<'a> for KeyVisitor<L> {
    type Value = ArrayKey<L>;

    fn expecting(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(stringify!($name))
    }

    fn visit_str<E>(self, value: &str) -> Result<ArrayKey<L>, E>
    where
        E: serde::de::Error,
    {
        let mut arr = GenericArray::default();
        hex::decode_to_slice(value, &mut arr[..]).map_err(E::custom)?;
        Ok(ArrayKey(arr))
    }
}

/// A possibly-empty password or key used to derive a store wrap key
#[derive(Clone)]
pub struct PassKey<'a>(Option<Cow<'a, str>>);

impl PassKey<'_> {
    /// Create a scoped reference to the passkey
    pub fn as_ref(&self) -> PassKey<'_> {
        PassKey(Some(Cow::Borrowed(&**self)))
    }

    pub(crate) fn is_none(&self) -> bool {
        self.0.is_none()
    }

    pub(crate) fn into_owned(self) -> PassKey<'static> {
        let mut slf = ManuallyDrop::new(self);
        let val = slf.0.take();
        PassKey(match val {
            None => None,
            Some(Cow::Borrowed(s)) => Some(Cow::Owned(s.to_string())),
            Some(Cow::Owned(s)) => Some(Cow::Owned(s)),
        })
    }
}

impl Debug for PassKey<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        if cfg!(test) {
            f.debug_tuple("PassKey").field(&*self).finish()
        } else {
            f.debug_tuple("PassKey").field(&"<secret>").finish()
        }
    }
}

impl Default for PassKey<'_> {
    fn default() -> Self {
        Self(None)
    }
}

impl Deref for PassKey<'_> {
    type Target = str;

    fn deref(&self) -> &str {
        match self.0.as_ref() {
            None => "",
            Some(s) => s.as_ref(),
        }
    }
}

impl Drop for PassKey<'_> {
    fn drop(&mut self) {
        self.zeroize();
    }
}

impl<'a> From<&'a str> for PassKey<'a> {
    fn from(inner: &'a str) -> Self {
        Self(Some(Cow::Borrowed(inner)))
    }
}

impl From<String> for PassKey<'_> {
    fn from(inner: String) -> Self {
        Self(Some(Cow::Owned(inner)))
    }
}

impl<'a> From<Option<&'a str>> for PassKey<'a> {
    fn from(inner: Option<&'a str>) -> Self {
        Self(inner.map(Cow::Borrowed))
    }
}

impl<'a, 'b> PartialEq<PassKey<'b>> for PassKey<'a> {
    fn eq(&self, other: &PassKey<'b>) -> bool {
        &**self == &**other
    }
}
impl Eq for PassKey<'_> {}

impl Zeroize for PassKey<'_> {
    fn zeroize(&mut self) {
        match self.0.take() {
            Some(Cow::Owned(mut s)) => {
                s.zeroize();
            }
            _ => (),
        }
    }
}

/// A heap-allocated, zeroized byte buffer
#[derive(Clone, Hash, PartialEq, Eq, PartialOrd, Ord, Zeroize)]
pub struct SecretBytes(Vec<u8>);

impl SecretBytes {
    pub fn new_with(len: usize, f: impl FnOnce(&mut [u8])) -> Self {
        let mut slf = Self::with_capacity(len);
        let mut buf = slf.as_buffer();
        buf.resize(len);
        f(buf.as_mut());
        slf
    }

    pub fn with_capacity(max_len: usize) -> Self {
        Self(Vec::with_capacity(max_len))
    }

    pub(crate) fn as_buffer(&mut self) -> SecretBytesMut<'_> {
        SecretBytesMut(&mut self.0)
    }

    pub fn from_slice(data: &[u8]) -> Self {
        let mut v = Vec::with_capacity(data.len());
        v.extend_from_slice(data);
        Self(v)
    }

    /// Try to convert the buffer value to a string reference
    pub fn as_opt_str(&self) -> Option<&str> {
        core::str::from_utf8(self.0.as_slice()).ok()
    }

    pub(crate) fn into_vec(mut self) -> Vec<u8> {
        // FIXME zeroize extra capacity?
        let mut v = Vec::new(); // note: no heap allocation for empty vec
        mem::swap(&mut v, &mut self.0);
        mem::forget(self);
        v
    }
}

impl Debug for SecretBytes {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        if cfg!(test) {
            f.debug_tuple("Secret")
                .field(&MaybeStr(self.0.as_slice()))
                .finish()
        } else {
            f.write_str("<secret>")
        }
    }
}

impl AsRef<[u8]> for SecretBytes {
    fn as_ref(&self) -> &[u8] {
        self.0.as_slice()
    }
}

impl Deref for SecretBytes {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        self.0.as_slice()
    }
}

impl Drop for SecretBytes {
    fn drop(&mut self) {
        self.zeroize();
    }
}

impl From<&[u8]> for SecretBytes {
    fn from(inner: &[u8]) -> Self {
        Self(inner.to_vec())
    }
}

impl From<&str> for SecretBytes {
    fn from(inner: &str) -> Self {
        Self(inner.as_bytes().to_vec())
    }
}

impl From<String> for SecretBytes {
    fn from(inner: String) -> Self {
        Self(inner.into_bytes())
    }
}

impl From<Vec<u8>> for SecretBytes {
    fn from(inner: Vec<u8>) -> Self {
        Self(inner)
    }
}

impl PartialEq<&[u8]> for SecretBytes {
    fn eq(&self, other: &&[u8]) -> bool {
        self.0.eq(other)
    }
}

impl PartialEq<Vec<u8>> for SecretBytes {
    fn eq(&self, other: &Vec<u8>) -> bool {
        self.0.eq(other)
    }
}

pub(crate) struct SecretBytesMut<'m>(&'m mut Vec<u8>);

impl SecretBytesMut<'_> {
    /// Securely extend the buffer without leaving copies
    #[inline]
    pub fn extend_from_slice(&mut self, data: &[u8]) {
        self.reserve(data.len());
        self.0.extend_from_slice(data);
    }

    fn truncate(&mut self, len: usize) {
        self.0.truncate(len);
    }

    /// Obtain a large-enough SecretBytes without creating unsafe copies of
    /// the contained data
    pub fn reserve(&mut self, extra: usize) {
        let len = self.0.len();
        if extra + len > self.0.capacity() {
            // allocate a new buffer and copy the secure data over
            let mut buf = Vec::with_capacity(extra + len);
            buf.extend_from_slice(&self.0[..]);
            mem::swap(&mut buf, &mut self.0);
            buf.zeroize()
        }
    }

    pub fn resize(&mut self, new_len: usize) {
        let len = self.len();
        if new_len > len {
            self.reserve(new_len - len);
            self.0.resize(new_len, 0u8);
        } else {
            self.0.truncate(new_len);
        }
    }
}

impl Buffer for SecretBytesMut<'_> {
    fn extend_from_slice(&mut self, other: &[u8]) -> Result<(), aead::Error> {
        self.extend_from_slice(other);
        Ok(())
    }

    fn truncate(&mut self, len: usize) {
        self.truncate(len);
    }
}

impl AsRef<[u8]> for SecretBytesMut<'_> {
    fn as_ref(&self) -> &[u8] {
        self.0.as_slice()
    }
}

impl AsMut<[u8]> for SecretBytesMut<'_> {
    fn as_mut(&mut self) -> &mut [u8] {
        self.0.as_mut_slice()
    }
}

/// A utility type for debug printing of byte strings
struct MaybeStr<'a>(&'a [u8]);

impl Debug for MaybeStr<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        if let Ok(sval) = core::str::from_utf8(self.0) {
            write!(f, "{:?}", sval)
        } else {
            f.write_char('<')?;
            for c in self.0 {
                f.write_fmt(format_args!("{:02x}", c))?;
            }
            f.write_char('>')?;
            Ok(())
        }
    }
}

pub trait WriteBuffer {
    fn extend_from_slice(&mut self, data: &[u8]) -> Result<(), Error> {
        if let Some(ext) = self.extend_buffer(data.len()) {
            ext.copy_from_slice(data);
            Ok(())
        } else {
            Err(err_msg!(Unexpected, "JWK buffer too small"))
        }
    }

    fn extend_buffer(&mut self, max_len: usize) -> Option<&mut [u8]>;

    fn truncate_by(&mut self, len: usize);
}

impl WriteBuffer for Vec<u8> {
    fn extend_buffer(&mut self, max_len: usize) -> Option<&mut [u8]> {
        let len = self.len();
        self.resize(len + max_len, 0u8);
        Some(&mut self[len..(len + max_len)])
    }

    fn truncate_by(&mut self, len: usize) {
        self.truncate(self.len().saturating_sub(len));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_write_vec() {
        let mut v = Vec::new();
        let mut b = &mut v as &mut dyn WriteBuffer;
        b.extend_from_slice(b"hello").unwrap();
        b.truncate_by(3);
        b.extend_from_slice(b"y").unwrap();
        assert_eq!(&v[..], b"hey");
    }

    #[test]
    fn test_maybe_str() {
        assert_eq!(format!("{:?}", MaybeStr(&[])), "\"\"");
        assert_eq!(format!("{:?}", MaybeStr(&[255, 0])), "<ff00>");
    }
}
