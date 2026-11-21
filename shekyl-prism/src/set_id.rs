// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

use core::cmp::Ordering;
use core::fmt;
use core::marker::PhantomData;

use prism_field::{ParameterSet, P248, P324, P376, P500, P664};
use prism_verify::set::Set;

/// Which parameter set some bytes are under: the identifier a tagged
/// key or signature starts with.
///
/// A parameter set is a prime `p = f·2^e − 1`, and the identifier is the
/// pair `(e, f)`. It is four bytes: `e` and then `f`, each a
/// little-endian `u16`. The five sets are
///
/// | set    | `e` | `f` | bytes         |
/// |--------|-----|-----|---------------|
/// | `p248` | 248 | 5   | `f8 00 05 00` |
/// | `p324` | 324 | 3   | `44 01 03 00` |
/// | `p376` | 376 | 65  | `78 01 41 00` |
/// | `p500` | 500 | 27  | `f4 01 1b 00` |
/// | `p664` | 664 | 17  | `98 02 11 00` |
///
/// The numbers are not written here a second time in code: they are
/// [`ParameterSet::E`] and the cofactor of each set.
///
/// # Valid by construction
///
/// A value of this type names one of the sets this library is built
/// with. [`Self::of`] gives the identifier of a set named as a type, and
/// [`Self::from_bytes`] refuses four bytes that name no built set, so
/// there is no `SetId` for a prime the library cannot verify under. Each
/// set has exactly one identifier and one encoding of it.
///
/// # What it does not say
///
/// The identifier in front of a key says which prime the bytes are
/// under. It does not say that the key's owner chose that prime, and
/// nothing in a signature binds it: see [`AcceptList`](crate::AcceptList)
/// for what a system that accepts more than one set has to do.
///
/// # Order
///
/// Identifiers increase with the prime. [`Ord`] compares `(e, f)` and
/// then the set, so the order agrees with [`Eq`]. An
/// [`AcceptList`](crate::AcceptList) keeps the sets it holds in this
/// order.
///
/// # There is no list of all of them
///
/// This interface has no constant, function or iterator that yields
/// every set. A system that accepts all five writes the five out, in its
/// own source, where a reviewer reads them.
///
/// ```compile_fail
/// let every = shekyl_prism::SetId::ALL;
/// ```
///
/// # Relation to the low-level name
///
/// The verifier crate has a run-time name for a set,
/// [`hazmat::verify::set::Set`](crate::hazmat::verify::set::Set), which tools
/// parse from text. A `SetId` is that value with the encoding added and
/// the list of all sets taken away. It holds a `Set` and nothing else.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct SetId(Set);

/// The pair `(e, f)` and the bare lengths of one built set.
///
/// Both come from [`ParameterSet`]. The identifier and the lengths then
/// have one definition each, and a set added to [`Set`] is named in one
/// match.
struct Parameters {
    e: u16,
    f: u16,
    public_key_len: usize,
    signature_len: usize,
}

/// `S` as [`Parameters`], with `e` and `f` checked to fit two bytes each.
const fn parameters_of<S: ParameterSet>() -> Parameters {
    let (e, f) = (S::E, S::COFACTOR.value());
    assert!(
        e <= u16::MAX as u32 && f <= u16::MAX as u64,
        "a set identifier holds e and f in two bytes each"
    );
    Parameters {
        e: e as u16,
        f: f as u16,
        public_key_len: S::PUBLIC_KEY_LEN,
        signature_len: S::SIGNATURE_LEN,
    }
}

/// [`Parameters`] of a set named as a value.
const fn parameters(set: Set) -> Parameters {
    match set {
        Set::P248 => parameters_of::<P248>(),
        Set::P324 => parameters_of::<P324>(),
        Set::P376 => parameters_of::<P376>(),
        Set::P500 => parameters_of::<P500>(),
        Set::P664 => parameters_of::<P664>(),
    }
}

/// The built set whose prime is `f·2^e − 1`, if there is one.
///
/// The sets are [`Set::ALL`]. A set added there is an identifier once
/// [`parameters`] names it, and that match does not compile until it does.
const fn built(e: u16, f: u16) -> Option<Set> {
    let mut index = 0;
    while index < Set::ALL.len() {
        let set = Set::ALL[index];
        let these = parameters(set);
        if these.e == e && these.f == f {
            return Some(set);
        }
        index += 1;
    }
    None
}

/// The identifier of `S`, found when `S` is known: at compile time.
struct Of<S>(PhantomData<S>);

impl<S: ParameterSet> Of<S> {
    const ID: SetId = {
        let these = parameters_of::<S>();
        match built(these.e, these.f) {
            Some(set) => SetId(set),
            None => panic!("this parameter set is not one the library is built with"),
        }
    };
}

impl SetId {
    /// The length of an identifier, in bytes.
    pub const LEN: usize = 4;

    /// The identifier of the set `S`.
    ///
    /// ```
    /// use shekyl_prism::{SetId, P248};
    ///
    /// let id = SetId::of::<P248>();
    /// assert_eq!((id.e(), id.f()), (248, 5));
    /// assert_eq!(id.to_bytes(), [0xf8, 0x00, 0x05, 0x00]);
    /// ```
    ///
    /// A type that implements [`ParameterSet`] and is not one of the five
    /// has no identifier, and a call with it does not compile.
    pub const fn of<S: ParameterSet>() -> Self {
        Of::<S>::ID
    }

    /// Read an identifier.
    ///
    /// # Errors
    ///
    /// [`UnknownSetId`], carrying the `e` and `f` read, when the bytes
    /// name no set this library is built with.
    ///
    /// ```
    /// use shekyl_prism::{SetId, UnknownSetId, P500};
    ///
    /// assert_eq!(SetId::from_bytes([0xf4, 0x01, 0x1b, 0x00]), Ok(SetId::of::<P500>()));
    /// // `e` and `f` the other way round name nothing.
    /// assert_eq!(
    ///     SetId::from_bytes([0x1b, 0x00, 0xf4, 0x01]),
    ///     Err(UnknownSetId { e: 27, f: 500 })
    /// );
    /// ```
    pub const fn from_bytes(bytes: [u8; Self::LEN]) -> Result<Self, UnknownSetId> {
        let e = u16::from_le_bytes([bytes[0], bytes[1]]);
        let f = u16::from_le_bytes([bytes[2], bytes[3]]);
        match built(e, f) {
            Some(set) => Ok(Self(set)),
            None => Err(UnknownSetId { e, f }),
        }
    }

    /// The identifier's encoding: `e` then `f`, each a little-endian
    /// `u16`. [`Self::from_bytes`] of it is this identifier.
    pub const fn to_bytes(self) -> [u8; Self::LEN] {
        let these = parameters(self.set());
        let (e, f) = (these.e.to_le_bytes(), these.f.to_le_bytes());
        [e[0], e[1], f[0], f[1]]
    }

    /// The exponent `e` of `p = f·2^e − 1`.
    pub const fn e(self) -> u16 {
        parameters(self.set()).e
    }

    /// The odd cofactor `f` of `p = f·2^e − 1`.
    pub const fn f(self) -> u16 {
        parameters(self.set()).f
    }

    /// The set's name, such as `p248`: [`ParameterSet::NAME`].
    pub fn name(self) -> &'static str {
        self.set().name()
    }

    /// The length of a tagged public key of this set, in bytes: the
    /// identifier and the bare key.
    pub const fn tagged_public_key_len(self) -> usize {
        Self::LEN + parameters(self.set()).public_key_len
    }

    /// The length of a tagged signature of this set, in bytes: the
    /// identifier and the bare signature.
    pub const fn tagged_signature_len(self) -> usize {
        Self::LEN + parameters(self.set()).signature_len
    }

    /// The low-level name the byte-level entry is selected by.
    pub(crate) const fn set(self) -> Set {
        self.0
    }
}

/// Increasing prime: `(e, f)`, then the set.
///
/// [`Eq`] is identity of the set. The pair orders distinct primes, and
/// the set is compared after it so two sets that shared a pair would
/// still compare as [`Eq`] says.
impl Ord for SetId {
    fn cmp(&self, other: &Self) -> Ordering {
        (self.e(), self.f())
            .cmp(&(other.e(), other.f()))
            .then_with(|| self.set().cmp(&other.set()))
    }
}

/// The order of [`Ord`].
impl PartialOrd for SetId {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// The set's name.
impl fmt::Display for SetId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

/// The set's name and the pair it stands for.
impl fmt::Debug for SetId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SetId({}: e={}, f={})", self.name(), self.e(), self.f())
    }
}

/// Four bytes that name no parameter set this library is built with.
///
/// The fields are what was read, so a log line or an error message can
/// say which prime the sender asked for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct UnknownSetId {
    /// The first two bytes, as a little-endian integer.
    pub e: u16,
    /// The last two bytes, as a little-endian integer.
    pub f: u16,
}

impl fmt::Display for UnknownSetId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "no parameter set has e = {} and f = {}", self.e, self.f)
    }
}

impl std::error::Error for UnknownSetId {}

#[cfg(test)]
mod tests {
    use super::*;

    /// Each of the five has its own identifier, each identifier decodes
    /// to its own set, the pair is the set's `E` and cofactor, and the
    /// identifiers increase with the prime.
    #[test]
    fn five_identifiers_distinct_and_each_decodes_to_its_own_set() {
        let ids = [
            (
                Set::P248,
                SetId::of::<P248>(),
                P248::E,
                P248::COFACTOR.value(),
            ),
            (
                Set::P324,
                SetId::of::<P324>(),
                P324::E,
                P324::COFACTOR.value(),
            ),
            (
                Set::P376,
                SetId::of::<P376>(),
                P376::E,
                P376::COFACTOR.value(),
            ),
            (
                Set::P500,
                SetId::of::<P500>(),
                P500::E,
                P500::COFACTOR.value(),
            ),
            (
                Set::P664,
                SetId::of::<P664>(),
                P664::E,
                P664::COFACTOR.value(),
            ),
        ];
        // The low-level tier may list the sets: this is every one of them.
        assert_eq!(ids.map(|(set, ..)| set), Set::ALL);
        for (index, (set, id, e, f)) in ids.into_iter().enumerate() {
            assert_eq!(id.set(), set);
            assert_eq!((u32::from(id.e()), u64::from(id.f())), (e, f));
            assert_eq!(SetId::from_bytes(id.to_bytes()), Ok(id));
            assert_eq!(id.name(), set.name());
            assert!(id.cmp(&id).is_eq());
            for (_, other, ..) in &ids[index + 1..] {
                assert_ne!(id, *other);
                assert_ne!(id.to_bytes(), other.to_bytes());
                assert!(id < *other);
            }
        }
    }

    /// The table in the type's documentation is what the code computes.
    #[test]
    fn documented_bytes() {
        assert_eq!(SetId::of::<P248>().to_bytes(), [0xf8, 0x00, 0x05, 0x00]);
        assert_eq!(SetId::of::<P324>().to_bytes(), [0x44, 0x01, 0x03, 0x00]);
        assert_eq!(SetId::of::<P376>().to_bytes(), [0x78, 0x01, 0x41, 0x00]);
        assert_eq!(SetId::of::<P500>().to_bytes(), [0xf4, 0x01, 0x1b, 0x00]);
        assert_eq!(SetId::of::<P664>().to_bytes(), [0x98, 0x02, 0x11, 0x00]);
    }
}
