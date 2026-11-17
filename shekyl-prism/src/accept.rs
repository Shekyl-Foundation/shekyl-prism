// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

use core::fmt;

use crate::SetId;

/// The parameter sets a system accepts: its policy, written out.
///
/// All five sets are first-class in this library. Which of them a system
/// accepts is that system's decision, and [`verify_tagged`] does not
/// make it: it takes this list and refuses every set that is not in it,
/// before it does any arithmetic.
///
/// ```
/// use shekyl_prism::{AcceptList, SetId, P248, P500};
///
/// let accept = AcceptList::new(&[SetId::of::<P500>()])?;
/// assert!(accept.contains(SetId::of::<P500>()));
/// assert!(!accept.contains(SetId::of::<P248>()));
/// # Ok::<(), shekyl_prism::EmptyAcceptList>(())
/// ```
///
/// # A list is as strong as its weakest member
///
/// Someone who wants to forge under a list attacks keys at the weakest
/// prime in it. Adding a larger prime to a list does not strengthen it,
/// and adding a smaller one weakens it for every key under that prime.
/// `docs/PARAMETER_SETS.md` in the repository records what each prime is
/// currently assessed at; read its table before writing a list.
///
/// # The tag does not choose for the key's owner
///
/// The identifier in a tagged key says which prime the bytes are under.
/// It does not say that whoever owns the key chose that prime. A system
/// that accepts more than one set must bind the set to the key where the
/// key is created or published, and must not let the identifier on an
/// incoming signature decide which set it is checked under. In Shekyl
/// that binding is in the output, and is Shekyl's to design.
///
/// The identifier is not part of what a signature covers.
/// [`verify_tagged`] refuses a key and a signature whose identifiers
/// differ, and bytes of one set under another set's identifier fail on
/// their length, because the five sets' lengths all differ. A system
/// that needs a signature bound to its set by more than that puts the
/// [`SetId`] in the message it signs.
///
/// # No default, and no "all"
///
/// There is one way to make a list, [`Self::new`], and it takes the sets
/// one by one. There is no default list:
///
/// ```compile_fail
/// let accept = shekyl_prism::AcceptList::default();
/// ```
///
/// no list of everything:
///
/// ```compile_fail
/// let accept = shekyl_prism::AcceptList::all();
/// ```
///
/// and no conversion from a collection:
///
/// ```compile_fail
/// use shekyl_prism::{AcceptList, SetId, P248};
/// let accept: AcceptList = vec![SetId::of::<P248>()].into();
/// ```
///
/// ```compile_fail
/// use shekyl_prism::{AcceptList, SetId, P248};
/// let accept: AcceptList = [SetId::of::<P248>()].into_iter().collect();
/// ```
///
/// A system that accepts all five writes the five out.
///
/// [`verify_tagged`]: crate::verify_tagged
#[derive(Clone, PartialEq, Eq)]
pub struct AcceptList {
    /// In the order first written, each once.
    sets: Vec<SetId>,
}

impl AcceptList {
    /// The list of exactly `sets`.
    ///
    /// A set written more than once is in the list once: two lists are
    /// equal when they were written with the same sets in the same order
    /// of first appearance.
    ///
    /// # Errors
    ///
    /// [`EmptyAcceptList`] when `sets` is empty. A list that accepts
    /// nothing is a mistake, and it fails here, where it is written,
    /// and not later as a refusal of every signature.
    ///
    /// ```
    /// use shekyl_prism::{AcceptList, EmptyAcceptList};
    ///
    /// assert_eq!(AcceptList::new(&[]), Err(EmptyAcceptList));
    /// ```
    pub fn new(sets: &[SetId]) -> Result<Self, EmptyAcceptList> {
        let mut kept: Vec<SetId> = Vec::with_capacity(sets.len());
        for set in sets {
            if !kept.contains(set) {
                kept.push(*set);
            }
        }
        if kept.is_empty() {
            return Err(EmptyAcceptList);
        }
        Ok(Self { sets: kept })
    }

    /// Whether `set` is in the list.
    pub fn contains(&self, set: SetId) -> bool {
        self.sets.contains(&set)
    }
}

/// The names of the sets in the list.
impl fmt::Debug for AcceptList {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("AcceptList")?;
        f.debug_list()
            .entries(self.sets.iter().map(|set| set.name()))
            .finish()
    }
}

/// An accept list was written with no set in it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct EmptyAcceptList;

impl fmt::Display for EmptyAcceptList {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("an accept list must hold at least one parameter set")
    }
}

impl std::error::Error for EmptyAcceptList {}
