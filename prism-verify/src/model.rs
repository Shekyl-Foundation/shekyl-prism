// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! Unique Montgomery coefficient for a response curve. Exceptional
//! automorphism classes are excluded from this research encoding.
//!
//! Rows 7 to 11 of the table in the crate documentation.
use crate::signature::ext_less;
use crate::VerifyError;
use prism_field::{Field, ParameterSet};

pub(crate) fn check<L: ParameterSet>(a: L::Ext) -> Result<(), VerifyError> {
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

#[cfg(test)]
mod tests {
    use super::check;
    use crate::signature::ext_less;
    use crate::VerifyError;
    use prism_field::{Field, ParameterSet, P248, P324, P376, P500, P664};

    /// A coefficient that is the lesser of `±A`, in no excluded class,
    /// with `A² − 4` not a square: the curve's points of order 2 are not
    /// all rational, and that is `Model`. Every other `Model` refusal is
    /// ruled out by how the coefficient is chosen, so this holds the
    /// refusal at the square root.
    fn a_non_square_discriminant_is_refused<L: ParameterSet>() {
        let mut found = 0;
        for re in 1u64..=8 {
            for im in 1u64..=8 {
                let a = L::ext_from_base(L::Base::from_u64(re))
                    .add(L::ext_i().mul(L::Ext::from_u64(im)));
                let square = a.square();
                let excluded = square.same(L::Ext::from_u64(4))
                    || square.same(L::Ext::from_u64(3))
                    || square.add(square).same(L::Ext::from_u64(9));
                if excluded
                    || ext_less::<L>(a.negate(), a)
                    || square.sub(L::Ext::from_u64(4)).is_square()
                {
                    continue;
                }
                assert_eq!(
                    check::<L>(a),
                    Err(VerifyError::Model),
                    "{} {re} + {im}i",
                    L::NAME
                );
                found += 1;
            }
        }
        assert!(found > 0, "{}: no such coefficient among the 64", L::NAME);
    }

    #[test]
    fn a_response_curve_without_rational_two_torsion_is_model_at_every_set() {
        a_non_square_discriminant_is_refused::<P248>();
        a_non_square_discriminant_is_refused::<P324>();
        a_non_square_discriminant_is_refused::<P376>();
        a_non_square_discriminant_is_refused::<P500>();
        a_non_square_discriminant_is_refused::<P664>();
    }
}
