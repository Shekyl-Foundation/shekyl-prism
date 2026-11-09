/* The kernel `fixed_degree_isogeny` builds on E0: an endomorphism of norm
 * u·(2^L − u), scaled by u^{-1} mod 2^{L+2}, on the canonical basis.
 * L = e − 2 and the chain is run with the extra 4-torsion. That is the
 * longest chain the reference evaluates on the full rational 2-power
 * torsion. Not linked into the Rust crates.
 */
#include <stdio.h>
#include <stdlib.h>

int gmp_printf(const char *fmt, ...) {
    (void)fmt;
    return 0;
}

#include "ec.h"
#include "encoded_sizes.h"
#include "endomorphism_action.h"
#include "hd.h"
#include "id2iso.h"
#include "intbig_internal.h"

static void print_ibz(const ibz_t *value) {
    ibz_t copy, ten, digit;
    ibz_init(&copy);
    ibz_init(&ten);
    ibz_init(&digit);
    ibz_copy(&copy, value);
    ibz_set(&ten, 10);
    char digits[512];
    int count = 0;
    if (ibz_is_zero(&copy)) {
        printf("0");
    } else {
        while (!ibz_is_zero(&copy)) {
            ibz_div(&copy, &digit, &copy, &ten);
            digits[count++] = (char)('0' + ibz_get(&digit));
        }
        while (count > 0) {
            putchar(digits[--count]);
        }
    }
    ibz_finalize(&copy);
    ibz_finalize(&ten);
    ibz_finalize(&digit);
}

static void hex_fp2(const fp2_t *value) {
    unsigned char bytes[FP2_ENCODED_BYTES];
    fp2_encode(bytes, value);
    for (int i = 0; i < FP2_ENCODED_BYTES; i++) {
        printf("%02x", bytes[i]);
    }
}

static void hex_point(const ec_point_t *point) {
    hex_fp2(&point->x);
    printf(" ");
    hex_fp2(&point->z);
}

int main(void) {
    const unsigned length = TORSION_EVEN_POWER - HD_extra_torsion;
    ibz_t two, u, pow, norm, inverse;
    ibz_init(&two);
    ibz_init(&u);
    ibz_init(&pow);
    ibz_init(&norm);
    ibz_init(&inverse);
    ibz_set(&two, 2);
    ibz_pow(&u, &two, 80);
    ibz_add(&u, &u, &ibz_const_one);
    for (int bump = 0; bump < 400; bump += 2) {
        if (ibz_probab_prime(&u, 8)) {
            break;
        }
        ibz_add(&u, &u, &two);
    }
    if (!ibz_probab_prime(&u, 16)) {
        fprintf(stderr, "no prime\n");
        return 1;
    }

    ibz_pow(&pow, &two, length);
    ibz_sub(&norm, &pow, &u);
    ibz_mul(&norm, &norm, &u);

    quat_alg_elem_t theta;
    quat_alg_elem_init(&theta);
    if (!quat_represent_integer(&theta, &norm, 1, &QUAT_represent_integer_params)) {
        fprintf(stderr, "represent failed\n");
        return 1;
    }

    ec_curve_t curve;
    copy_curve(&curve, &CURVE_E0);
    ec_curve_normalize_A24(&curve);
    ec_basis_t domain;
    copy_basis(&domain, &BASIS_EVEN);
    ec_dbl_iter_basis(&domain, TORSION_EVEN_POWER - length - HD_extra_torsion, &domain, &curve);

    ibz_mul(&pow, &pow, &ibz_const_two);
    ibz_mul(&pow, &pow, &ibz_const_two);
    if (!ibz_invmod(&inverse, &u, &pow)) {
        fprintf(stderr, "inverse failed\n");
        return 1;
    }
    for (int i = 0; i < 4; i++) {
        ibz_mul(&theta.coord[i], &theta.coord[i], &inverse);
    }
    ec_basis_t image;
    copy_basis(&image, &domain);
    endomorphism_application_even_basis(&image, 0, &curve, &theta, (int)(length + HD_extra_torsion));

    theta_couple_curve_t domain_product;
    domain_product.E1 = curve;
    domain_product.E2 = curve;
    theta_kernel_couple_points_t kernel;
    copy_bases_to_kernel(&kernel, &domain, &image);
    theta_couple_curve_t codomain;
    int ok = theta_chain_compute_and_eval(length, &domain_product, &kernel, true, &codomain, NULL, 0);
    fprintf(stderr, "chain %d\n", ok);
    if (!ok) {
        return 1;
    }

    printf("length %u\n", length);
    printf("u ");
    print_ibz(&u);
    printf("\n");
    printf("P ");
    hex_point(&domain.P);
    printf("\nQ ");
    hex_point(&domain.Q);
    printf("\nD ");
    hex_point(&domain.PmQ);
    printf("\nPs ");
    hex_point(&image.P);
    printf("\nQs ");
    hex_point(&image.Q);
    printf("\nDs ");
    hex_point(&image.PmQ);
    printf("\n");

    quat_alg_elem_finalize(&theta);
    ibz_finalize(&two);
    ibz_finalize(&u);
    ibz_finalize(&pow);
    ibz_finalize(&norm);
    ibz_finalize(&inverse);
    return 0;
}
