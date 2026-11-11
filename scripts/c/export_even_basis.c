/* Copyright (c) 2025-2026, The Shekyl Foundation
 *
 * All rights reserved.
 * BSD-3-Clause
 */

/* Print the canonical order-2^e basis the C tree builds from A.
 *
 * stdout is three lines, P, Q, and PmQ, each the affine x-coordinate in
 * the tree's fp2 encoding. In this tree Q is difference_point(P, R) and
 * PmQ is R, the point with x(R) = -x(P) - A. Not linked into the Rust crates.
 */
#include <ctype.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

int gmp_printf(const char *fmt, ...) {
    (void)fmt;
    return 0;
}

#include "ec.h"
#include "encoded_sizes.h"

static int nibble(char text) {
    if (text >= '0' && text <= '9') {
        return text - '0';
    }
    if (text >= 'a' && text <= 'f') {
        return text - 'a' + 10;
    }
    if (text >= 'A' && text <= 'F') {
        return text - 'A' + 10;
    }
    return -1;
}

static int parse_hex(unsigned char *out, const char *text, int length) {
    if ((int)strlen(text) != length * 2) {
        return 0;
    }
    for (int i = 0; i < length; i++) {
        int high = nibble(text[2 * i]);
        int low = nibble(text[2 * i + 1]);
        if (high < 0 || low < 0) {
            return 0;
        }
        out[i] = (unsigned char)((high << 4) | low);
    }
    return 1;
}

static void print_point(const ec_point_t *point) {
    ec_point_t affine = *point;
    ec_normalize_point(&affine);
    unsigned char bytes[FP2_ENCODED_BYTES];
    fp2_encode(bytes, &affine.x);
    for (int i = 0; i < FP2_ENCODED_BYTES; i++) {
        printf("%02x", bytes[i]);
    }
    printf("\n");
}

int main(int argc, char **argv) {
    unsigned char bytes[FP2_ENCODED_BYTES];
    char *end = NULL;
    long exponent = argc == 3 ? strtol(argv[2], &end, 10) : 0;
    if (argc != 3 || !parse_hex(bytes, argv[1], FP2_ENCODED_BYTES) || end == argv[2] || *end != '\0'
        || exponent < 2 || exponent > 1000) {
        fprintf(stderr, "usage: %s <A-hex> <even-exponent>\n", argv[0]);
        return 2;
    }
    ec_curve_t curve;
    memset(&curve, 0, sizeof(curve));
    if (!fp2_decode(&curve.A, bytes)) {
        fprintf(stderr, "A does not decode\n");
        return 1;
    }
    fp2_set_one(&curve.C);
    ec_basis_t basis;
    ec_curve_to_basis_2f_to_hint(&basis, &curve, (int)exponent);
    print_point(&basis.P);
    print_point(&basis.Q);
    print_point(&basis.PmQ);
    return 0;
}
