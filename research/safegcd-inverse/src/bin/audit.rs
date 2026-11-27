//! Non-inlined instances of the CT inverse for the asm audit
//! (`objdump -d --no-show-raw-insn target/release/audit`, symbols `audit_inv_*`).
use safegcd_inverse::*;
use std::hint::black_box;

#[no_mangle]
#[inline(never)]
pub extern "C" fn audit_inv_p500_q(x: &S62<17>, m: &S62<17>, out: &mut Inverse<17>) {
    *out = inv_mod_ct(x, m, 37);
}
#[no_mangle]
#[inline(never)]
pub extern "C" fn audit_inv_p500_n(x: &S62<5>, m: &S62<5>, out: &mut Inverse<5>) {
    *out = inv_mod_ct(x, m, 10);
}

fn main() {
    let m = S62::<17>::from_u64_limbs(&[0xffff_ffff_ffff_fff1, 7, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
    let x = S62::<17>::from_u64_limbs(&[12345, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
    let mut o = inv_mod_ct(&x, &m, 1);
    audit_inv_p500_q(black_box(&x), black_box(&m), &mut o);
    black_box(o.ok);
    let m5 = S62::<5>::from_u64_limbs(&[0xffff_ffff_ffff_fff1, 7, 0, 0, 0]);
    let x5 = S62::<5>::from_u64_limbs(&[12345, 0, 0, 0, 0]);
    let mut o5 = inv_mod_ct(&x5, &m5, 1);
    audit_inv_p500_n(black_box(&x5), black_box(&m5), &mut o5);
    black_box(o5.ok);
}
