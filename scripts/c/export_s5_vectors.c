/* Copyright (c) 2025-2026, The Shekyl Foundation
 *
 * All rights reserved.
 * BSD-3-Clause
 */

/* Algorithm 2 of ePrint 2026/443 at the printed torsion. The chain exponent
 * stays e' = e - 2 and q = 2^{2e'-1} - s^2; the scalar is t = s^{-1} mod 2^e.
 * The response is a basis of E_sig[2^e] on the least Montgomery model of its
 * isomorphism class. Not linked into the Rust crates.
 *
 * keygen <scratch>   seed is 48 bytes on stdin; scratch receives the secret
 * sign <scratch> <s> s is decimal; stdout is the public response
 */
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

/* The reference build's randombytes_init ignores its seed. This generator
 * is the one the harness links, so a 48-byte seed reproduces a key. */
static uint64_t rng_state[6];

static uint64_t rng_word(void) {
    uint64_t oldest = rng_state[0];
    uint64_t mixed = rng_state[1];
    for (int index = 0; index < 5; index++) {
        rng_state[index] = rng_state[index + 1];
    }
    mixed ^= mixed << 23;
    mixed ^= mixed >> 17;
    rng_state[5] = mixed ^ oldest ^ (oldest >> 26);
    return rng_state[5] + oldest;
}

void randombytes_init(unsigned char *entropy_input, unsigned char *personalization_string, int security_strength) {
    (void)personalization_string;
    (void)security_strength;
    memcpy(rng_state, entropy_input, sizeof(rng_state));
}

int randombytes(unsigned char *buffer, unsigned long long length) {
    while (length > 0) {
        uint64_t word = rng_word();
        unsigned long long take = length < 8 ? length : 8;
        memcpy(buffer, &word, take);
        buffer += take;
        length -= take;
    }
    return 0;
}

int gmp_printf(const char *fmt, ...) {
    (void)fmt;
    return 0;
}

#include "ec.h"
#include "encoded_sizes.h"
#include "hd.h"
#include "id2iso.h"
#include "internal.h"
#include "quaternion_data.h"
#include "rng.h"
#include "signature.h"
#include "torsion_constants.h"

typedef struct affine_pair {
    fp2_t px;
    fp2_t py;
    fp2_t qx;
    fp2_t qy;
} affine_pair_t;

typedef struct mont_model {
    fp2_t a;
    affine_pair_t points;
} mont_model_t;

static void affine_from_jac(fp2_t *x, fp2_t *y, const jac_point_t *point) {
    fp2_t inverse;
    fp2_t inverse_square;
    fp2_copy(&inverse, &point->z);
    fp2_inv(&inverse);
    fp2_sqr(&inverse_square, &inverse);
    fp2_mul(x, &point->x, &inverse_square);
    fp2_mul(&inverse, &inverse_square, &inverse);
    fp2_mul(y, &point->y, &inverse);
}

static void hex_fp2(const fp2_t *value) {
    unsigned char bytes[FP2_ENCODED_BYTES];
    fp2_encode(bytes, value);
    for (int i = 0; i < FP2_ENCODED_BYTES; i++) {
        printf("%02x", bytes[i]);
    }
}

/* <_{F_{p^2}} as in prism-verify::signature::ext_less: real part as an
 * integer in [0, p), then the imaginary part. */
static int fp_le_less(const fp_t *left, const fp_t *right) {
    unsigned char left_bytes[FP_ENCODED_BYTES];
    unsigned char right_bytes[FP_ENCODED_BYTES];
    fp_encode(left_bytes, left);
    fp_encode(right_bytes, right);
    for (int i = (int)FP_ENCODED_BYTES - 1; i >= 0; i--) {
        if (left_bytes[i] < right_bytes[i]) {
            return 1;
        }
        if (left_bytes[i] > right_bytes[i]) {
            return 0;
        }
    }
    return 0;
}

static int fp2_ext_less(const fp2_t *left, const fp2_t *right) {
    if (fp_is_equal(&left->re, &right->re)) {
        return fp_le_less(&left->im, &right->im);
    }
    return fp_le_less(&left->re, &right->re);
}

static void imag_unit(fp2_t *out) {
    fp2_set_zero(out);
    fp_set_one(&out->im);
}

static void negate_model(mont_model_t *out, const mont_model_t *in) {
    fp2_t i;
    imag_unit(&i);
    fp2_neg(&out->a, &in->a);
    fp2_neg(&out->points.px, &in->points.px);
    fp2_mul(&out->points.py, &in->points.py, &i);
    fp2_neg(&out->points.qx, &in->points.qx);
    fp2_mul(&out->points.qy, &in->points.qy, &i);
}

/* Move the order-2 point (alpha, 0) to (0, 0). A' = (3 alpha + A) / lambda
 * with lambda^2 = alpha^2 - 1, and (x, y) |-> ((x - alpha)/lambda,
 * y / lambda^{3/2}). */
static int move_two_torsion_at(mont_model_t *out, const mont_model_t *in, const fp2_t *alpha) {
    fp2_t lambda;
    fp2_t lambda_sqrt;
    fp2_t scale;
    fp2_t one;
    fp2_t three;
    fp2_t inv_lambda;

    fp2_set_one(&one);
    fp2_sqr(&lambda, alpha);
    fp2_sub(&lambda, &lambda, &one);
    if (!fp2_is_square(&lambda)) {
        return 0;
    }
    fp2_copy(&lambda_sqrt, &lambda);
    fp2_sqrt(&lambda_sqrt);
    fp2_copy(&lambda, &lambda_sqrt);

    fp2_set_small(&three, 3);
    fp2_mul(&out->a, &three, alpha);
    fp2_add(&out->a, &out->a, &in->a);
    fp2_copy(&inv_lambda, &lambda);
    fp2_inv(&inv_lambda);
    fp2_mul(&out->a, &out->a, &inv_lambda);

    fp2_copy(&lambda_sqrt, &lambda);
    if (!fp2_is_square(&lambda_sqrt)) {
        return 0;
    }
    fp2_sqrt(&lambda_sqrt);
    fp2_mul(&scale, &lambda, &lambda_sqrt);
    fp2_inv(&scale);

    fp2_sub(&out->points.px, &in->points.px, alpha);
    fp2_mul(&out->points.px, &out->points.px, &inv_lambda);
    fp2_mul(&out->points.py, &in->points.py, &scale);

    fp2_sub(&out->points.qx, &in->points.qx, alpha);
    fp2_mul(&out->points.qx, &out->points.qx, &inv_lambda);
    fp2_mul(&out->points.qy, &in->points.qy, &scale);
    return 1;
}

static int two_torsion_roots(fp2_t *alpha, fp2_t *beta, const fp2_t *a) {
    fp2_t disc;
    fp2_t two;
    fp2_t inv_two;
    fp2_t sqrt_disc;

    fp2_sqr(&disc, a);
    fp2_set_small(&two, 4);
    fp2_sub(&disc, &disc, &two);
    if (!fp2_is_square(&disc)) {
        return 0;
    }
    fp2_copy(&sqrt_disc, &disc);
    fp2_sqrt(&sqrt_disc);
    fp2_set_small(&two, 2);
    fp2_copy(&inv_two, &two);
    fp2_inv(&inv_two);

    fp2_neg(alpha, a);
    fp2_add(alpha, alpha, &sqrt_disc);
    fp2_mul(alpha, alpha, &inv_two);

    fp2_neg(beta, a);
    fp2_sub(beta, beta, &sqrt_disc);
    fp2_mul(beta, beta, &inv_two);
    return 1;
}

static int choose_canonical_model(mont_model_t *out, const mont_model_t *in) {
    mont_model_t candidates[6];
    fp2_t alpha;
    fp2_t beta;
    int count = 0;

    candidates[count++] = *in;
    negate_model(&candidates[count++], in);

    if (!two_torsion_roots(&alpha, &beta, &in->a)) {
        return 0;
    }
    if (!move_two_torsion_at(&candidates[count], in, &alpha)) {
        return 0;
    }
    negate_model(&candidates[count + 1], &candidates[count]);
    count += 2;
    if (!move_two_torsion_at(&candidates[count], in, &beta)) {
        return 0;
    }
    negate_model(&candidates[count + 1], &candidates[count]);
    count += 2;

    int best = 0;
    for (int i = 1; i < count; i++) {
        if (fp2_ext_less(&candidates[i].a, &candidates[best].a)) {
            best = i;
        }
    }
    *out = candidates[best];
    return 1;
}

static int set_decimal(ibz_t *out, const char *text) {
    ibz_t ten;
    ibz_t digit;
    ibz_init(&ten);
    ibz_init(&digit);
    ibz_set(&ten, 10);
    ibz_set(out, 0);
    if (*text == '\0') {
        ibz_finalize(&ten);
        ibz_finalize(&digit);
        return 0;
    }
    for (const char *cursor = text; *cursor != '\0'; cursor++) {
        if (*cursor < '0' || *cursor > '9') {
            ibz_finalize(&ten);
            ibz_finalize(&digit);
            return 0;
        }
        ibz_set(&digit, (int)(*cursor - '0'));
        ibz_mul(out, out, &ten);
        ibz_add(out, out, &digit);
    }
    ibz_finalize(&ten);
    ibz_finalize(&digit);
    return 1;
}

static int read_seed(unsigned char seed[48]) {
    size_t got = fread(seed, 1, 48, stdin);
    return got == 48;
}

static int keygen(const char *scratch) {
    unsigned char seed[48];
    if (!read_seed(seed)) {
        fprintf(stderr, "seed must be 48 bytes\n");
        return 1;
    }
    randombytes_init(seed, NULL, 256);

    public_key_t public_key;
    secret_key_t secret;
    secret_key_init(&secret);
    if (!protocols_keygen(&public_key, &secret)) {
        fprintf(stderr, "keygen failed\n");
        secret_key_finalize(&secret);
        return 1;
    }

    unsigned char encoded[SECRETKEY_BYTES];
    secret_key_to_bytes(encoded, &secret, &public_key);
    FILE *file = fopen(scratch, "wb");
    if (file == NULL || fwrite(encoded, 1, SECRETKEY_BYTES, file) != SECRETKEY_BYTES) {
        fprintf(stderr, "scratch write failed\n");
        if (file != NULL) {
            fclose(file);
        }
        secret_key_finalize(&secret);
        return 1;
    }
    fclose(file);

    ec_curve_t curve = public_key.curve;
    ec_normalize_curve(&curve);
    printf("A ");
    hex_fp2(&curve.A);
    printf("\n");
    secret_key_finalize(&secret);
    return 0;
}

static int sign(const char *scratch, const char *scalar_text) {
    unsigned char seed[48];
    if (!read_seed(seed)) {
        fprintf(stderr, "seed must be 48 bytes\n");
        return 1;
    }
    randombytes_init(seed, NULL, 256);

    FILE *file = fopen(scratch, "rb");
    unsigned char encoded[SECRETKEY_BYTES];
    if (file == NULL || fread(encoded, 1, SECRETKEY_BYTES, file) != SECRETKEY_BYTES) {
        fprintf(stderr, "scratch read failed\n");
        if (file != NULL) {
            fclose(file);
        }
        return 1;
    }
    fclose(file);

    public_key_t public_key;
    secret_key_t secret;
    secret_key_init(&secret);
    secret_key_from_bytes(&secret, &public_key, encoded);

    const int chain_exponent = TORSION_EVEN_POWER - 2;
    const int torsion_exponent = TORSION_EVEN_POWER;
    ibz_t scalar;
    ibz_t two;
    ibz_t power;
    ibz_t degree;
    ibz_t inverse;
    ibz_t modulus;
    ibz_init(&scalar);
    ibz_init(&two);
    ibz_init(&power);
    ibz_init(&degree);
    ibz_init(&inverse);
    ibz_init(&modulus);
    ibz_set(&two, 2);
    if (!set_decimal(&scalar, scalar_text) || ibz_is_even(&scalar) || ibz_is_zero(&scalar)) {
        fprintf(stderr, "s must be a positive odd decimal\n");
        return 1;
    }
    ibz_pow(&power, &two, 2 * chain_exponent - 1);
    ibz_mul(&degree, &scalar, &scalar);
    if (ibz_cmp(&degree, &power) >= 0) {
        fprintf(stderr, "s is not below 2^{e'-1}\n");
        return 1;
    }
    ibz_sub(&degree, &power, &degree);
    ibz_pow(&modulus, &two, torsion_exponent);
    if (!ibz_invmod(&inverse, &scalar, &modulus)) {
        fprintf(stderr, "s is not invertible mod 2^e\n");
        return 1;
    }

    quat_left_ideal_t ideal;
    quat_left_ideal_init(&ideal);
    ec_curve_t codomain;
    ec_basis_t image;
    ec_basis_t canonical;
    ibz_mat_2x2_t matrix;
    ibz_mat_2x2_init(&matrix);
    int produced = 0;
    for (int attempt = 0; attempt < 16 && !produced; attempt++) {
        int sampled = 0;
        for (int draw = 0; draw < 64 && !sampled; draw++) {
            sampled = quat_sampling_random_ideal_O0_given_norm(
                &ideal, &degree, 1, &QUAT_represent_integer_params, NULL);
        }
        if (!sampled) {
            continue;
        }
        quat_lideal_inter(&ideal, &secret.secret_ideal, &ideal, &QUATALG_PINFTY);
        if (!dim2id2iso_arbitrary_isogeny_evaluation(&image, &codomain, &ideal)) {
            fprintf(stderr, "ideal did not evaluate, retrying\n");
            continue;
        }
        ec_curve_normalize_A24(&codomain);
        ec_curve_to_basis_2f_to_hint(&canonical, &codomain, TORSION_EVEN_POWER);
        change_of_basis_matrix_tate(
            &matrix, &image, &canonical, &codomain, TORSION_EVEN_POWER);
        ibz_2x2_mul_mod(
            &matrix, &matrix, &secret.mat_BAcan_to_BA0_two, &TORSION_PLUS_2POWER);
        for (int row = 0; row < 2; row++) {
            for (int column = 0; column < 2; column++) {
                ibz_mul(&matrix[row][column], &matrix[row][column], &inverse);
                ibz_mod(&matrix[row][column], &matrix[row][column], &TORSION_PLUS_2POWER);
            }
        }
        if (!matrix_application_even_basis(
                &canonical, &codomain, &matrix, TORSION_EVEN_POWER)) {
            fprintf(stderr, "basis application failed, retrying\n");
            continue;
        }
        if (test_basis_order_twof(&canonical, &codomain, torsion_exponent) != (int)0xFFFFFFFF) {
            fprintf(stderr, "image is not a 2^e basis, retrying\n");
            continue;
        }
        produced = 1;
    }
    if (!produced) {
        fprintf(stderr, "no response\n");
        return 1;
    }

    /* Okeya–Sakurai, via lift_basis: Q's sign is the one for which
     * x(P−Q) is the basis difference. */
    ec_basis_t basis = canonical;
    ec_curve_t curve = codomain;
    jac_point_t lifted_p;
    jac_point_t lifted_q;
    if (lift_basis(&lifted_p, &lifted_q, &basis, &curve) == 0) {
        fprintf(stderr, "y recovery failed\n");
        return 1;
    }

    mont_model_t raw;
    affine_from_jac(&raw.points.px, &raw.points.py, &lifted_p);
    affine_from_jac(&raw.points.qx, &raw.points.qy, &lifted_q);
    fp2_copy(&raw.a, &curve.A);

    mont_model_t chosen;
    if (!choose_canonical_model(&chosen, &raw)) {
        fprintf(stderr, "canonical model failed\n");
        return 1;
    }

    /* Emit the verification-key basis from the secret, unswapped: the
     * response is t*sigma of exactly (canonical_basis.P, canonical_basis.Q),
     * so the exported pair must be that pair (up to one global sign from
     * lift_basis, which [-1]-composes sigma and is harmless). The earlier
     * Q<->PmQ swap exported (P, P-Q) instead and silently broke the
     * correspondence the verifier's Kani construction depends on. */
    ec_curve_t vk_curve = secret.curve;
    ec_basis_t vk_basis = secret.canonical_basis;
    jac_point_t vk_p;
    jac_point_t vk_q;
    if (lift_basis(&vk_p, &vk_q, &vk_basis, &vk_curve) == 0) {
        fprintf(stderr, "vk y recovery failed\n");
        return 1;
    }
    fp2_t vk_px;
    fp2_t vk_py;
    fp2_t vk_qx;
    fp2_t vk_qy;
    fp2_t vk_dx;
    affine_from_jac(&vk_px, &vk_py, &vk_p);
    affine_from_jac(&vk_qx, &vk_qy, &vk_q);
    {
        fp2_t inv;
        fp2_copy(&inv, &vk_basis.PmQ.z);
        fp2_inv(&inv);
        fp2_mul(&vk_dx, &vk_basis.PmQ.x, &inv);
    }

    printf("E ");
    hex_fp2(&chosen.a);
    printf("\nPx ");
    hex_fp2(&chosen.points.px);
    printf("\nPy ");
    hex_fp2(&chosen.points.py);
    printf("\nQx ");
    hex_fp2(&chosen.points.qx);
    printf("\nQy ");
    hex_fp2(&chosen.points.qy);
    printf("\nVkPx ");
    hex_fp2(&vk_px);
    printf("\nVkPy ");
    hex_fp2(&vk_py);
    printf("\nVkQx ");
    hex_fp2(&vk_qx);
    printf("\nVkQy ");
    hex_fp2(&vk_qy);
    printf("\nVkDx ");
    hex_fp2(&vk_dx);
    printf("\n");

    quat_left_ideal_finalize(&ideal);
    ibz_mat_2x2_finalize(&matrix);
    ibz_finalize(&scalar);
    ibz_finalize(&two);
    ibz_finalize(&power);
    ibz_finalize(&degree);
    ibz_finalize(&inverse);
    ibz_finalize(&modulus);
    secret_key_finalize(&secret);
    return 0;
}

int main(int argc, char **argv) {
    if (argc == 3 && strcmp(argv[1], "keygen") == 0) {
        return keygen(argv[2]);
    }
    if (argc == 4 && strcmp(argv[1], "sign") == 0) {
        return sign(argv[2], argv[3]);
    }
    fprintf(stderr, "usage: %s keygen <scratch> | sign <scratch> <s>\n", argv[0]);
    return 2;
}
