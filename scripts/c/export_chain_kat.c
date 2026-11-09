/* Print one theta-chain known answer from the pinned reference.

 * Build against one level. Arguments: the chain length n. The domain is
 * E0 × E0 and the kernel is the precomputed basis of order 2^{n+2}, which
 * is the order `theta_chain_compute_and_eval_verify` asks for when
 * extra_torsion is set. The point pushed through is (P, 0).
 *
 * Not linked into the Rust crates.
 */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

int gmp_printf(const char *fmt, ...) {
    (void)fmt;
    return 0;
}

#include "ec.h"
#include "encoded_sizes.h"
#include "hd.h"

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

static void hex_couple(const theta_couple_point_t *point) {
    hex_point(&point->P1);
    printf(" ");
    hex_point(&point->P2);
}

static int parse_hex(unsigned char *out, const char *text, int length) {
    if ((int)strlen(text) != length * 2) {
        return 0;
    }
    for (int i = 0; i < length; i++) {
        unsigned value;
        if (sscanf(text + 2 * i, "%02x", &value) != 1) {
            return 0;
        }
        out[i] = (unsigned char)value;
    }
    return 1;
}

int main(int argc, char **argv) {
    unsigned n = argc > 1 ? (unsigned)atoi(argv[1]) : 4;
    ec_curve_t first;
    ec_curve_t second;
    ec_curve_init(&first);
    ec_curve_init(&second);
    if (argc > 2) {
        unsigned char bytes[FP2_ENCODED_BYTES];
        if (!parse_hex(bytes, argv[2], FP2_ENCODED_BYTES) || !fp2_decode(&second.A, bytes)) {
            fprintf(stderr, "coefficient\n");
            return 1;
        }
    }
    ec_basis_t basis_1;
    ec_basis_t basis_2;
    ec_curve_to_basis_2f_to_hint(&basis_1, &first, (int)(n + 2));
    ec_curve_to_basis_2f_to_hint(&basis_2, &second, (int)(n + 2));

    theta_couple_curve_t domain;
    domain.E1 = first;
    domain.E2 = second;
    theta_kernel_couple_points_t kernel;
    copy_bases_to_kernel(&kernel, &basis_1, &basis_2);

    theta_couple_point_t image;
    image.P1 = basis_1.P;
    ec_point_init(&image.P2);

    printf("n %u\n", n);
    printf("curves ");
    hex_fp2(&first.A);
    printf(" ");
    hex_fp2(&first.C);
    printf(" ");
    hex_fp2(&second.A);
    printf(" ");
    hex_fp2(&second.C);
    printf("\n");
    printf("T1 ");
    hex_couple(&kernel.T1);
    printf("\n");
    printf("T2 ");
    hex_couple(&kernel.T2);
    printf("\n");
    printf("diff ");
    hex_couple(&kernel.T1m2);
    printf("\n");
    printf("point ");
    hex_couple(&image);
    printf("\n");

    theta_couple_curve_t codomain;
    int ok = theta_chain_compute_and_eval_verify(
        n, &domain, &kernel, 1, &codomain, &image, 1);
    printf("ok %d\n", ok);
    if (!ok) {
        return 1;
    }
    printf("codomain ");
    hex_fp2(&codomain.E1.A);
    printf(" ");
    hex_fp2(&codomain.E1.C);
    printf(" ");
    hex_fp2(&codomain.E2.A);
    printf(" ");
    hex_fp2(&codomain.E2.C);
    printf("\n");
    printf("image ");
    hex_couple(&image);
    printf("\n");
    return 0;
}
