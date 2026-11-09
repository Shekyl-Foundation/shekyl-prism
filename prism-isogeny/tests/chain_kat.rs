// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! The length-160 chain in `testdata/chain_lvl1.txt` is
//! `theta_chain_compute_and_eval` on the graph of an endomorphism of `E0`,
//! generated from the pinned C tree. The codomain and the image are compared
//! projectively.

use prism_field::{Field, Level, Level1};
use prism_isogeny::chain::chain;
use prism_isogeny::gluing::{Couple, Kernel};
use prism_isogeny::montgomery::{Montgomery, Xz};

type Ext = <Level1 as Level>::Ext;

fn element(hex: &str) -> Ext {
    let bytes: Vec<u8> = (0..hex.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(&hex[index..index + 2], 16).unwrap())
        .collect();
    Level1::ext_decode(&bytes).expect("canonical field element")
}

fn point(words: &[&str]) -> Xz<Ext> {
    Xz {
        x: element(words[0]),
        z: element(words[1]),
    }
}

fn couple(words: &[&str]) -> Couple<Ext> {
    Couple {
        first: point(&words[0..2]),
        second: point(&words[2..4]),
    }
}

fn same_curve(left: Montgomery<Ext>, right: Montgomery<Ext>) -> bool {
    left.a.mul(right.c).same(right.a.mul(left.c))
}

fn same_point(left: Xz<Ext>, right: Xz<Ext>) -> bool {
    left.x.mul(right.z).same(right.x.mul(left.z))
}

#[test]
fn the_length_160_chain_matches_the_reference() {
    check(include_str!("../testdata/chain_lvl1.txt"), true);
}

#[test]
fn the_length_160_chain_without_extra_torsion_matches_the_reference() {
    check(include_str!("../testdata/chain_lvl1_order.txt"), false);
}

#[test]
fn the_fixed_degree_kernel_on_the_starting_curve_splits() {
    let text = include_str!("../testdata/e0_fixed_lvl1.txt");
    let mut lines = text.lines().filter(|line| !line.is_empty());
    let length: u32 = lines
        .next()
        .unwrap()
        .split_whitespace()
        .nth(1)
        .unwrap()
        .parse()
        .unwrap();
    let _u = lines.next().unwrap();
    let read = |line: &str| {
        let words: Vec<&str> = line.split_whitespace().skip(1).collect();
        point(&words)
    };
    let domain_p = read(lines.next().unwrap());
    let domain_q = read(lines.next().unwrap());
    let domain_d = read(lines.next().unwrap());
    let image_p = read(lines.next().unwrap());
    let image_q = read(lines.next().unwrap());
    let image_d = read(lines.next().unwrap());
    let zero = Montgomery::normalized(Ext::zero());
    let kernel = Kernel {
        t1: Couple {
            first: domain_p,
            second: image_p,
        },
        t2: Couple {
            first: domain_q,
            second: image_q,
        },
        difference: Couple {
            first: domain_d,
            second: image_d,
        },
    };
    let pushed = Couple {
        first: domain_p,
        second: Xz::infinity(),
    };
    chain(
        length,
        [zero, zero],
        kernel,
        true,
        &[pushed],
        Level1::ext_i(),
        true,
    )
    .expect("the fixed-degree kernel splits");
}

fn check(text: &str, extra_torsion: bool) {
    let mut lines = text.lines().filter(|line| !line.is_empty());
    let header = lines.next().unwrap();
    assert!(header.starts_with("length 160 "));
    let curves = lines.next().unwrap();
    let t1 = lines.next().unwrap();
    let t2 = lines.next().unwrap();
    let difference = lines.next().unwrap();
    let point_line = lines.next().unwrap();
    assert_eq!(lines.next().unwrap(), "chain 1");
    let codomain = lines.next().unwrap();
    let image = lines.next().unwrap();

    let curve_words: Vec<&str> = curves.split_whitespace().skip(1).collect();
    let domain = [
        Montgomery {
            a: element(curve_words[0]),
            c: element(curve_words[1]),
        },
        Montgomery {
            a: element(curve_words[2]),
            c: element(curve_words[3]),
        },
    ];
    let kernel = Kernel {
        t1: couple(&t1.split_whitespace().skip(1).collect::<Vec<_>>()),
        t2: couple(&t2.split_whitespace().skip(1).collect::<Vec<_>>()),
        difference: couple(&difference.split_whitespace().skip(1).collect::<Vec<_>>()),
    };
    let input = couple(&point_line.split_whitespace().skip(1).collect::<Vec<_>>());
    let result = chain(
        160,
        domain,
        kernel,
        extra_torsion,
        &[input],
        Level1::ext_i(),
        true,
    )
    .expect("the reference chain splits");

    let codomain_words: Vec<&str> = codomain.split_whitespace().skip(1).collect();
    let expected = [
        Montgomery {
            a: element(codomain_words[0]),
            c: element(codomain_words[1]),
        },
        Montgomery {
            a: element(codomain_words[2]),
            c: element(codomain_words[3]),
        },
    ];
    assert!(same_curve(result.codomain[0], expected[0]));
    assert!(same_curve(result.codomain[1], expected[1]));
    let image_words: Vec<&str> = image.split_whitespace().skip(1).collect();
    let expected_image = couple(&image_words);
    assert!(same_point(result.images[0].first, expected_image.first));
    assert!(same_point(result.images[0].second, expected_image.second));
}
