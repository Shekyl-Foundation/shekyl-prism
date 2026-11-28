// Copyright (c) 2026, The Shekyl Foundation
// All rights reserved.
// BSD-3-Clause

//! Public test seeds. The published reference is used only to mint fixtures.
use prism_signer::{SecretSeed, Signer};
use prism_verify::set::CommittedSet;
use quorum_approval_experiment::{fixed_pair_digest, Payment, Policy};
use shekyl_prism::{SetId, P248, P376, P500};
use std::{env, fs, path::PathBuf};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let build = PathBuf::from(
        env::args_os()
            .nth(1)
            .ok_or("pass the reference build directory")?,
    );
    for (level, set, backend) in [
        (CommittedSet::P248, SetId::of::<P248>(), "lvl1"),
        (CommittedSet::P376, SetId::of::<P376>(), "lvl3"),
        (CommittedSet::P500, SetId::of::<P500>(), "lvl5"),
    ] {
        let mut members = Vec::new();
        let mut public = Vec::new();
        for seed in 1..=3 {
            let signer = Signer::new(
                build.join(format!("prism_signer_{backend}")),
                level,
                SecretSeed::from_bytes([seed; 48]),
            );
            let mut key = set.to_bytes().to_vec();
            key.extend(signer.public_key()?);
            public.push(key);
            members.push(signer);
        }
        let policy = Policy::enroll(set, [&public[0], &public[1], &public[2]])?;
        let digest = Payment::fixture().digest(&policy);
        let dir = PathBuf::from("fixtures").join(set.name());
        fs::create_dir_all(&dir)?;
        fs::write(dir.join("enrollment.bin"), public.concat())?;
        for (i, signer) in members.iter().enumerate() {
            let mut approval = vec![i as u8];
            approval.extend(&public[i]);
            approval.extend(set.to_bytes());
            approval.extend(signer.sign(&digest)?);
            fs::write(dir.join(format!("{i}.bin")), approval)?;
        }
        let mut fixed = set.to_bytes().to_vec();
        fixed.extend(members[0].sign(&fixed_pair_digest(&digest, [0, 1]))?);
        fs::write(dir.join("alice-fixed-ab.sig"), fixed)?;
        eprintln!("generated {}", set.name());
    }
    Ok(())
}
