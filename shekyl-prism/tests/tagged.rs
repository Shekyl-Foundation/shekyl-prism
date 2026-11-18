// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! The tag layer: the identifier in front of a key or a signature, the
//! accept list, and the order in which `verify_tagged` refuses.
//!
//! None of these tests needs an honest signature to make its point: a
//! tag-layer refusal is decided from the first four bytes of each input
//! and the list. Where bare bytes are needed they are zeros of the right
//! length, which the byte-level entry refuses, so "the tag layer let it
//! through" shows as a wrapped `VerifyError`.
//!
//! That tagged equals bare on the honest vectors, the negatives and the
//! refusals is held in `support/entries.rs`, on every input the other
//! tests of this crate build.

use prism_vectors::corpus::WithCorpus;
use shekyl_prism::{
    verify_tagged, AcceptList, Container, EmptyAcceptList, ParameterSet, PublicKey, SetId,
    Signature, TaggedError, UnknownSetId, VerifyError, P248, P324, P376, P500, P664,
};

/// One set: its identifier and its two bare lengths.
#[derive(Clone, Copy, Debug)]
struct Row {
    id: SetId,
    key_len: usize,
    signature_len: usize,
}

fn row<S: ParameterSet>() -> Row {
    Row {
        id: SetId::of::<S>(),
        key_len: S::PUBLIC_KEY_LEN,
        signature_len: S::SIGNATURE_LEN,
    }
}

/// The five sets, written out: the interface has no list of them.
fn rows() -> [Row; 5] {
    [
        row::<P248>(),
        row::<P324>(),
        row::<P376>(),
        row::<P500>(),
        row::<P664>(),
    ]
}

fn ids() -> [SetId; 5] {
    rows().map(|row| row.id)
}

/// `tag ‖ bare`.
fn under(tag: [u8; 4], bare: &[u8]) -> Vec<u8> {
    [&tag[..], bare].concat()
}

/// A tagged key of this set whose bare part is zeros.
fn key(row: Row) -> Vec<u8> {
    under(row.id.to_bytes(), &vec![0; row.key_len])
}

/// A tagged signature of this set whose bare part is zeros.
fn signature(row: Row) -> Vec<u8> {
    under(row.id.to_bytes(), &vec![0; row.signature_len])
}

fn list(sets: &[SetId]) -> AcceptList {
    AcceptList::new(sets).expect("a list of at least one set")
}

/// Whether the tag layer let the input through to the bare checks.
fn passed_the_tag_layer(result: Result<SetId, TaggedError>) -> bool {
    matches!(result, Ok(_) | Err(TaggedError::Verify(_)))
}

const MESSAGE: &[u8] = b"tagged";

#[test]
fn the_bare_lengths_name_the_set() {
    // What stops bytes of one set being presented under another set's
    // identifier is that the lengths differ. A set added with a length
    // another set has fails here, and reopens the question of binding
    // the identifier in the challenge (docs/VERIFIER_SPEC.md).
    let rows = rows();
    for (index, one) in rows.iter().enumerate() {
        for other in &rows[index + 1..] {
            assert_ne!(one.key_len, other.key_len, "{} {}", one.id, other.id);
            assert_ne!(
                one.signature_len, other.signature_len,
                "{} {}",
                one.id, other.id
            );
        }
        assert_eq!(one.id.tagged_public_key_len(), 4 + one.key_len);
        assert_eq!(one.id.tagged_signature_len(), 4 + one.signature_len);
    }
}

#[test]
fn every_ordered_pair_of_sets_is_a_mismatch_unless_equal() {
    let accept_all = list(&ids());
    let (mut equal, mut unequal) = (0, 0);
    for a in rows() {
        for b in rows() {
            let both = list(&[a.id, b.id]);
            for accept in [&both, &accept_all] {
                let result = verify_tagged(accept, &key(a), MESSAGE, &signature(b));
                if a.id == b.id {
                    assert!(passed_the_tag_layer(result), "{} {result:?}", a.id);
                } else {
                    assert_eq!(
                        result,
                        Err(TaggedError::Mismatch {
                            public_key: a.id,
                            signature: b.id
                        })
                    );
                }
            }
            if a.id == b.id {
                equal += 1;
            } else {
                unequal += 1;
            }
        }
    }
    assert_eq!((equal, unequal), (5, 20));
}

#[test]
fn a_tag_over_another_sets_bytes_is_refused_as_length() {
    let length = Err(TaggedError::Verify(VerifyError::Length));
    let mut cases = 0;
    for a in rows() {
        let accept = list(&[a.id]);
        for b in rows() {
            if a.id == b.id {
                continue;
            }
            // The key is another set's length.
            let other_key = under(a.id.to_bytes(), &vec![0; b.key_len]);
            assert_eq!(
                verify_tagged(&accept, &other_key, MESSAGE, &signature(a)),
                length
            );
            // The signature is.
            let other_signature = under(a.id.to_bytes(), &vec![0; b.signature_len]);
            assert_eq!(
                verify_tagged(&accept, &key(a), MESSAGE, &other_signature),
                length
            );
            // Both are: a key and signature of `b`, retagged whole.
            assert_eq!(
                verify_tagged(&accept, &other_key, MESSAGE, &other_signature),
                length
            );
            cases += 1;
        }
    }
    assert_eq!(cases, 20);
}

/// An honest key and signature of `B` under the identifier of `A`.
fn retagged<B: WithCorpus>(a: Row) -> usize {
    let mut cases = 0;
    for primary in B::PRIMARY {
        let vector = primary.vector();
        let key = under(a.id.to_bytes(), &vector.public_key().unwrap());
        let signature = under(a.id.to_bytes(), &vector.signature().unwrap());
        assert_eq!(
            verify_tagged(&list(&ids()), &key, vector.message().unwrap(), &signature),
            Err(TaggedError::Verify(VerifyError::Length)),
            "{} retagged {}",
            B::NAME,
            a.id
        );
        cases += 1;
    }
    cases
}

#[test]
fn an_honest_signature_retagged_is_refused_as_length() {
    // The identifier is not in the challenge. The length is what refuses.
    let mut cases = 0;
    for a in rows() {
        if a.id != SetId::of::<P248>() {
            cases += retagged::<P248>(a);
        }
        if a.id != SetId::of::<P376>() {
            cases += retagged::<P376>(a);
        }
        if a.id != SetId::of::<P500>() {
            cases += retagged::<P500>(a);
        }
    }
    // Six honest vectors, each under the four other identifiers.
    assert_eq!(cases, 24);
}

#[test]
fn the_list_refuses_what_it_does_not_hold_and_nothing_else() {
    assert_eq!(AcceptList::new(&[]), Err(EmptyAcceptList));
    for row in rows() {
        let others: Vec<SetId> = ids().into_iter().filter(|id| *id != row.id).collect();
        assert_eq!(others.len(), 4);
        let without = list(&others);
        assert!(!without.contains(row.id));
        assert_eq!(
            verify_tagged(&without, &key(row), MESSAGE, &signature(row)),
            Err(TaggedError::NotAccepted(row.id))
        );
        // A list of one other set alone refuses it too.
        for other in others {
            assert_eq!(
                verify_tagged(&list(&[other]), &key(row), MESSAGE, &signature(row)),
                Err(TaggedError::NotAccepted(row.id))
            );
        }
        for with in [list(&[row.id]), list(&ids())] {
            assert!(with.contains(row.id));
            assert!(passed_the_tag_layer(verify_tagged(
                &with,
                &key(row),
                MESSAGE,
                &signature(row)
            )));
        }
    }
}

#[test]
fn a_list_is_the_sets_in_it_whatever_order_and_however_often_written() {
    let [a, b, ..] = ids();
    assert_eq!(list(&[a, a, b, a]), list(&[a, b]));
    assert_eq!(list(&[a, a]), list(&[a]));
    assert_eq!(list(&[b, a]), list(&[a, b]));
    assert_ne!(list(&[a]), list(&[a, b]));
    assert_eq!(
        format!("{:?}", list(&[b, a, b])),
        r#"AcceptList["p248", "p324"]"#
    );
}

/// Pairs `(e, f)` that name no set, each made from the built ones.
fn unknown_pairs() -> Vec<(u16, u16)> {
    let pairs: Vec<(u16, u16)> = ids().iter().map(|id| (id.e(), id.f())).collect();
    let mut unknown = vec![(0, 0), (0xffff, 0xffff)];
    for &(e, f) in &pairs {
        // `e` and `f` the other way round; `e` one off.
        unknown.extend([(f, e), (e + 1, f), (e - 1, f)]);
        // A built `e` with another set's `f`.
        unknown.extend(
            pairs
                .iter()
                .filter(|other| other.1 != f)
                .map(|other| (e, other.1)),
        );
    }
    unknown
}

fn tag_of((e, f): (u16, u16)) -> [u8; 4] {
    let (e, f) = (e.to_le_bytes(), f.to_le_bytes());
    [e[0], e[1], f[0], f[1]]
}

#[test]
fn an_identifier_that_names_nothing_is_refused_with_the_pair_read() {
    let unknown = unknown_pairs();
    // Two fixed, and for each of five sets: three, and four other `f`.
    assert_eq!(unknown.len(), 2 + 5 * (3 + 4));
    let good = row::<P248>();
    let accept = list(&ids());
    for pair in unknown {
        let tag = tag_of(pair);
        let id = UnknownSetId {
            e: pair.0,
            f: pair.1,
        };
        assert_eq!(SetId::from_bytes(tag), Err(id), "{pair:?}");
        let refused = |container| Err(TaggedError::UnknownSet { container, id });
        // On the key, with a good signature; on the signature, with a
        // good key. The bare part is a real length, so only the tag is
        // at fault.
        assert_eq!(
            verify_tagged(
                &accept,
                &under(tag, &vec![0; good.key_len]),
                MESSAGE,
                &signature(good)
            ),
            refused(Container::PublicKey)
        );
        assert_eq!(
            verify_tagged(
                &accept,
                &key(good),
                MESSAGE,
                &under(tag, &vec![0; good.signature_len])
            ),
            refused(Container::Signature)
        );
        assert_eq!(
            PublicKey::<P248>::from_tagged(&under(tag, &vec![0; good.key_len])).unwrap_err(),
            TaggedError::UnknownSet {
                container: Container::PublicKey,
                id
            }
        );
        assert_eq!(
            Signature::<P248>::from_tagged(&under(tag, &vec![0; good.signature_len])).unwrap_err(),
            TaggedError::UnknownSet {
                container: Container::Signature,
                id
            }
        );
    }
}

#[test]
fn an_input_shorter_than_an_identifier_is_refused_as_too_short() {
    let accept = list(&ids());
    let mut cases = 0;
    for row in rows() {
        for len in 0..4 {
            let short = &row.id.to_bytes()[..len];
            assert_eq!(
                verify_tagged(&accept, short, MESSAGE, &signature(row)),
                Err(TaggedError::TooShort {
                    container: Container::PublicKey,
                    len
                })
            );
            assert_eq!(
                verify_tagged(&accept, &key(row), MESSAGE, short),
                Err(TaggedError::TooShort {
                    container: Container::Signature,
                    len
                })
            );
            cases += 2;
        }
    }
    // Lengths 0 to 3, for the key and for the signature, at five sets.
    assert_eq!(cases, 40);
    for len in 0..4 {
        let short = &SetId::of::<P248>().to_bytes()[..len];
        assert_eq!(
            PublicKey::<P248>::from_tagged(short).unwrap_err(),
            TaggedError::TooShort {
                container: Container::PublicKey,
                len
            }
        );
        assert_eq!(
            Signature::<P248>::from_tagged(short).unwrap_err(),
            TaggedError::TooShort {
                container: Container::Signature,
                len
            }
        );
    }
}

#[test]
fn the_tag_layer_refuses_in_its_stated_order() {
    let [a, b, c, ..] = rows();
    let nothing = tag_of((1, 1));
    let nothing_else = tag_of((2, 2));
    let unknown = |container, e| {
        Err(TaggedError::UnknownSet {
            container,
            id: UnknownSetId { e, f: e },
        })
    };
    let short = |container, len| Err(TaggedError::TooShort { container, len });
    let accept = list(&[a.id, b.id]);

    // Within "too short": both are, and the key is named.
    assert_eq!(
        verify_tagged(&accept, &[], MESSAGE, &[0; 3]),
        short(Container::PublicKey, 0)
    );
    // Too short before an unknown identifier on the key: the signature
    // is short and the key's identifier names nothing.
    for len in 0..4 {
        assert_eq!(
            verify_tagged(&accept, &under(nothing, &[]), MESSAGE, &vec![0xff; len]),
            short(Container::Signature, len)
        );
    }
    // The key's unknown identifier before the signature's.
    assert_eq!(
        verify_tagged(
            &accept,
            &under(nothing, &[]),
            MESSAGE,
            &under(nothing_else, &[])
        ),
        unknown(Container::PublicKey, 1)
    );
    // The signature's unknown identifier before a mismatch: its
    // identifier is not the key's, and it is refused as naming nothing.
    assert_eq!(
        verify_tagged(&accept, &key(a), MESSAGE, &under(nothing_else, &[])),
        unknown(Container::Signature, 2)
    );
    // A mismatch before the list: neither set is in the list.
    assert_eq!(
        verify_tagged(&list(&[c.id]), &key(a), MESSAGE, &signature(b)),
        Err(TaggedError::Mismatch {
            public_key: a.id,
            signature: b.id
        })
    );
    // The list before the bare bytes: the set is not in the list, and
    // the bare parts are another set's length, or empty.
    let not_accepted = Err(TaggedError::NotAccepted(c.id));
    assert_eq!(
        verify_tagged(
            &accept,
            &under(c.id.to_bytes(), &vec![0; a.key_len]),
            MESSAGE,
            &under(c.id.to_bytes(), &vec![0; a.signature_len])
        ),
        not_accepted
    );
    assert_eq!(
        verify_tagged(&accept, &c.id.to_bytes(), MESSAGE, &c.id.to_bytes()),
        not_accepted
    );
    // And with the set in the list, those same bytes reach the bare
    // checks and fail on their length.
    assert_eq!(
        verify_tagged(&list(&[c.id]), &c.id.to_bytes(), MESSAGE, &c.id.to_bytes()),
        Err(TaggedError::Verify(VerifyError::Length))
    );
}

/// The typed readers at `S`, on a container tagged with each set.
fn typed_readers<S: ParameterSet>() -> (usize, usize) {
    let expected = SetId::of::<S>();
    let (mut right, mut wrong) = (0, 0);
    for row in rows() {
        // The bare part is the tagging set's length, so for another set
        // it is not `S`'s length either: the tag is refused first.
        let key = PublicKey::<S>::from_tagged(&key(row)).unwrap_err();
        let signature = Signature::<S>::from_tagged(&signature(row)).unwrap_err();
        if row.id == expected {
            assert!(matches!(key, TaggedError::Verify(_)), "{key:?}");
            assert!(matches!(signature, TaggedError::Verify(_)), "{signature:?}");
            right += 1;
        } else {
            let wrong_set = TaggedError::WrongSet {
                expected,
                found: row.id,
            };
            assert_eq!(key, wrong_set);
            assert_eq!(signature, wrong_set);
            // Also when what follows is exactly `S`'s length.
            assert_eq!(
                PublicKey::<S>::from_tagged(&under(row.id.to_bytes(), &vec![0; S::PUBLIC_KEY_LEN]))
                    .unwrap_err(),
                wrong_set
            );
            wrong += 1;
        }
    }
    (right, wrong)
}

#[test]
fn a_typed_reader_refuses_another_sets_identifier_before_the_bare_bytes() {
    let counts = [
        typed_readers::<P248>(),
        typed_readers::<P324>(),
        typed_readers::<P376>(),
        typed_readers::<P500>(),
        typed_readers::<P664>(),
    ];
    assert_eq!(counts, [(1, 4); 5]);
}

/// Tagged bytes read and written again are the same bytes.
fn round_trip<S: WithCorpus>() -> usize {
    let id = SetId::of::<S>();
    let accept = list(&[id]);
    let mut vectors = 0;
    for primary in S::PRIMARY {
        let vector = primary.vector();
        let key = PublicKey::<S>::from_bytes(&vector.public_key().unwrap()).unwrap();
        let signature = Signature::<S>::from_bytes(&vector.signature().unwrap()).unwrap();

        let tagged_key = key.to_tagged();
        assert_eq!(tagged_key.len(), PublicKey::<S>::TAGGED_LEN);
        assert_eq!(tagged_key.len(), id.tagged_public_key_len());
        assert_eq!(tagged_key, under(id.to_bytes(), key.as_bytes()));
        let read = PublicKey::<S>::from_tagged(&tagged_key).unwrap();
        assert_eq!(read, key);
        assert_eq!(read.to_tagged(), tagged_key);

        let tagged_signature = signature.to_tagged();
        assert_eq!(tagged_signature.len(), Signature::<S>::TAGGED_LEN);
        assert_eq!(tagged_signature.len(), id.tagged_signature_len());
        assert_eq!(tagged_signature, under(id.to_bytes(), signature.as_bytes()));
        let read = Signature::<S>::from_tagged(&tagged_signature).unwrap();
        assert_eq!(read, signature);
        assert_eq!(read.to_tagged(), tagged_signature);

        // What the typed writers give is what the tagged entry accepts.
        assert_eq!(
            verify_tagged(
                &accept,
                &tagged_key,
                vector.message().unwrap(),
                &tagged_signature
            ),
            Ok(id)
        );
        // One byte more is not the container.
        let mut longer = tagged_signature.clone();
        longer.push(0);
        assert_eq!(
            verify_tagged(&accept, &tagged_key, vector.message().unwrap(), &longer),
            Err(TaggedError::Verify(VerifyError::Length))
        );
        assert_eq!(
            Signature::<S>::from_tagged(&longer).unwrap_err(),
            TaggedError::Verify(VerifyError::Length)
        );
        vectors += 1;
    }
    vectors
}

#[test]
fn tagged_bytes_read_and_written_again_are_the_same_bytes() {
    assert_eq!(
        round_trip::<P248>() + round_trip::<P376>() + round_trip::<P500>(),
        6
    );
}

#[test]
fn an_identifier_has_one_encoding() {
    // Four bytes, and of the 2^32 values five are identifiers. Every
    // identifier's bytes decode to it and encode back, and no two share
    // bytes; a tagged value is then unique when the bare one is.
    for id in ids() {
        assert_eq!(SetId::from_bytes(id.to_bytes()), Ok(id));
        assert_eq!(tag_of((id.e(), id.f())), id.to_bytes());
        assert_eq!(ids().iter().filter(|other| **other == id).count(), 1);
    }
    assert_eq!(format!("{}", SetId::of::<P376>()), "p376");
    assert_eq!(
        format!("{:?}", SetId::of::<P376>()),
        "SetId(p376: e=376, f=65)"
    );
}

#[test]
fn an_error_names_the_error_under_it_and_only_where_there_is_one() {
    use std::error::Error;

    // `source` is a property of the value. `Verify` and `UnknownSet`
    // carry the error under them; a refusal the tag layer made itself
    // carries none. A new variant does not compile here until it says which.
    let p248 = SetId::of::<P248>();
    let p500 = SetId::of::<P500>();
    let pair = UnknownSetId { e: 27, f: 500 };
    let inner = VerifyError::Composite;
    let refusals = [
        TaggedError::TooShort {
            container: Container::Signature,
            len: 3,
        },
        TaggedError::UnknownSet {
            container: Container::PublicKey,
            id: pair,
        },
        TaggedError::Mismatch {
            public_key: p248,
            signature: p500,
        },
        TaggedError::WrongSet {
            expected: p500,
            found: p248,
        },
        TaggedError::NotAccepted(p248),
        TaggedError::Verify(inner),
    ];
    for error in refusals {
        match &error {
            TaggedError::Verify(found) => {
                assert_eq!(
                    error
                        .source()
                        .and_then(|source| source.downcast_ref::<VerifyError>()),
                    Some(found)
                );
            }
            TaggedError::UnknownSet { id, .. } => {
                assert_eq!(
                    error
                        .source()
                        .and_then(|source| source.downcast_ref::<UnknownSetId>()),
                    Some(id)
                );
            }
            TaggedError::TooShort { .. }
            | TaggedError::Mismatch { .. }
            | TaggedError::WrongSet { .. }
            | TaggedError::NotAccepted(_) => {
                assert!(error.source().is_none(), "{error:?} names a source");
            }
        }
    }
}

#[test]
fn every_refusal_says_what_was_refused_and_carries_what_it_read() {
    // The message is what an operator reads beside a refused peer. Each
    // names its input, and the set, the pair or the length it carries.
    let p248 = SetId::of::<P248>();
    let p500 = SetId::of::<P500>();

    assert_eq!(Container::PublicKey.to_string(), "public key");
    assert_eq!(Container::Signature.to_string(), "signature");
    assert_eq!(
        UnknownSetId { e: 27, f: 500 }.to_string(),
        "no parameter set has e = 27 and f = 500"
    );
    assert_eq!(
        EmptyAcceptList.to_string(),
        "an accept list must hold at least one parameter set"
    );

    let said = [
        (
            TaggedError::TooShort {
                container: Container::Signature,
                len: 3,
            },
            "the tagged signature is 3 bytes, shorter than a set identifier",
        ),
        (
            TaggedError::UnknownSet {
                container: Container::PublicKey,
                id: UnknownSetId { e: 27, f: 500 },
            },
            "the tagged public key: no parameter set has e = 27 and f = 500",
        ),
        (
            TaggedError::Mismatch {
                public_key: p248,
                signature: p500,
            },
            "the public key is tagged p248 and the signature p500",
        ),
        (
            TaggedError::WrongSet {
                expected: p500,
                found: p248,
            },
            "tagged p248, and p500 was expected",
        ),
        (
            TaggedError::NotAccepted(p248),
            "p248 is not in the accept list",
        ),
    ];
    for (error, text) in said {
        assert_eq!(error.to_string(), text);
    }

    // The verifier's refusal is passed on in the verifier's words.
    let inner = VerifyError::Composite;
    assert!(!inner.to_string().is_empty());
    assert_eq!(TaggedError::Verify(inner).to_string(), inner.to_string());
}
