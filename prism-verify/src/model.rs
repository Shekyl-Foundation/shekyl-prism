// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! Unique Montgomery coefficient for a response curve. Exceptional
//! automorphism classes are excluded from this research encoding.
use crate::signature::ext_less;
use crate::VerifyError;
use prism_field::{Field, Level};

pub(crate) fn check<L: Level>(a: L::Ext) -> Result<(), VerifyError> {
    let square = a.square();
    if square.same(L::Ext::from_u64(4)) {
        return Err(VerifyError::Singular);
    }
    // j=0: A²=3. j=1728: A=0 or 2A²=9. A least coefficient alone
    // cannot normalize the extra automorphisms fixing these curves.
    if a.is_zero()
        || square.same(L::Ext::from_u64(3))
        || square.add(square).same(L::Ext::from_u64(9))
    {
        return Err(VerifyError::Model);
    }
    if ext_less::<L>(a.negate(), a) {
        return Err(VerifyError::Model);
    }
    let disc = square
        .sub(L::Ext::from_u64(4))
        .sqrt()
        .ok_or(VerifyError::Model)?;
    let half = L::Ext::from_u64(2).inv().ok_or(VerifyError::Model)?;
    for root in [disc, disc.negate()] {
        let alpha = a.negate().add(root).mul(half);
        let lambda2 = alpha.square().sub(L::Ext::one());
        if let Some(lambda) = lambda2.sqrt() {
            let moved = alpha
                .mul(L::Ext::from_u64(3))
                .add(a)
                .mul(lambda.inv().ok_or(VerifyError::Model)?);
            if ext_less::<L>(moved, a) || ext_less::<L>(moved.negate(), a) {
                return Err(VerifyError::Model);
            }
        }
    }
    Ok(())
}
