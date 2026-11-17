// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

//! The verifier's error for a verdict of the committed refusal vectors.
//!
//! `prism-vectors` does not depend on the verifier, so its file names a
//! refusal by a type of its own. This is the mapping from one to the
//! other. A test that pins a row's error states the error itself and
//! holds this function to it.

use prism_vectors::refusals::Verdict;
use prism_verify::VerifyError;

/// The error the verifier must return for a row of that verdict.
pub fn error_of(verdict: Verdict) -> VerifyError {
    match verdict {
        Verdict::Kernel => VerifyError::Kernel,
        Verdict::Codomain => VerifyError::Codomain,
    }
}
