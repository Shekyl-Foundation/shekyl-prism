// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

// The integer crate's committed vectors, given to the C reference.
//
// Reads prism-int/tests/data/vectors.tsv (written by scripts/int_vectors.py
// from Python's integers, and read by the crate's own tests) and gives the
// operands of each line to the reference's ibz_* function for that
// operation, where it has one. A result that differs is printed with its
// line number and the program exits 1. So the three agree: Python, the
// Rust crate, and the reference, on every convention docs/SIGNER_SPEC.md
// 2.1.4 marks as one a replay depends on.
//
// Built and run by scripts/check_int_conventions.sh against the pinned
// reference build. The file's values are test operands, not secrets.

#define _GNU_SOURCE
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include <quaternion.h>
#include "internal.h"
#include "hnf_internal.h"

#define MAX_FIELDS 32

static long line_number;
static long failures;

// What was compared, by the reference's function.
enum {
    C_ADD, C_SUB, C_MUL, C_CMP, C_DIV, C_DIV_FLOOR, C_MOD, C_ROUNDED_DIV, C_CENTERED_MOD,
    C_DIV_2EXP, C_GET, C_BITSIZE, C_IS_ODD, C_GCD, C_INVMOD, C_SQRT, C_SQRT_FLOOR, C_LEGENDRE,
    C_COUNT
};
static const char *const NAMES[C_COUNT] = {
    "ibz_add", "ibz_sub", "ibz_mul", "ibz_cmp", "ibz_div", "ibz_div_floor", "ibz_mod",
    "ibz_rounded_div", "ibz_centered_mod", "ibz_div_2exp", "ibz_get", "ibz_bitsize",
    "ibz_is_odd", "ibz_gcd", "ibz_invmod", "ibz_sqrt", "ibz_sqrt_floor", "ibz_legendre",
};
static long compared[C_COUNT];

static void
fail(int what, const char *detail)
{
    fprintf(stderr, "line %ld: %s differs (%s)\n", line_number, NAMES[what], detail);
    failures++;
}

// An integer of the file: 0x… after an optional -. Returns 0 for a field
// that is a refusal's name.
static int
parse(ibz_t *out, const char *field)
{
    const char *digits = field;
    int negative = 0;
    if (*digits == '-') {
        negative = 1;
        digits++;
    }
    if (digits[0] != '0' || digits[1] != 'x')
        return 0;
    if (ibz_set_from_str(out, digits + 2, 16) != 1) {
        fprintf(stderr, "line %ld: not an integer: %s\n", line_number, field);
        exit(2);
    }
    if (negative)
        ibz_neg(out, out);
    return 1;
}

// The reference's value is the file's.
static void
same(int what, const ibz_t *got, const char *field)
{
    ibz_t want;
    ibz_init(&want);
    compared[what]++;
    if (!parse(&want, field))
        fail(what, "the file refuses, the reference has a value");
    else if (ibz_cmp(got, &want) != 0)
        fail(what, "value");
    ibz_finalize(&want);
}

static void
same_int(int what, long got, const char *field)
{
    compared[what]++;
    if (got != strtol(field, NULL, 10))
        fail(what, "number");
}

int
main(int argc, char **argv)
{
    if (argc != 2) {
        fprintf(stderr, "usage: %s <vectors.tsv>\n", argv[0]);
        return 2;
    }
    FILE *file = fopen(argv[1], "r");
    if (file == NULL) {
        perror(argv[1]);
        return 2;
    }
    ibz_t a, b, x, y;
    ibz_init(&a);
    ibz_init(&b);
    ibz_init(&x);
    ibz_init(&y);
    char *line = NULL;
    size_t capacity = 0;
    ssize_t length;
    while ((length = getline(&line, &capacity, file)) > 0) {
        line_number++;
        if (line[0] == '#')
            continue;
        if (line[length - 1] == '\n')
            line[length - 1] = '\0';
        char *fields[MAX_FIELDS];
        int count = 0;
        for (char *rest = line, *field; (field = strsep(&rest, "\t")) != NULL && count < MAX_FIELDS;)
            fields[count++] = field;
        int equals = 0;
        while (equals < count && strcmp(fields[equals], "=") != 0)
            equals++;
        if (count < 4 || equals == count) {
            fprintf(stderr, "line %ld: not a line of the file\n", line_number);
            return 2;
        }
        const char *op = fields[0];
        char **operand = fields + 2;
        char **result = fields + equals + 1;
        int results = count - equals - 1;
        parse(&a, operand[0]);

        if (strcmp(op, "ring") == 0) {
            parse(&b, operand[1]);
            // The reference's integers grow, so it has a value where the
            // file says the fixed width overflows. Those are skipped.
            ibz_add(&x, &a, &b);
            if (strcmp(result[0], "overflow") != 0)
                same(C_ADD, &x, result[0]);
            ibz_sub(&x, &a, &b);
            if (strcmp(result[1], "overflow") != 0)
                same(C_SUB, &x, result[1]);
            ibz_mul(&x, &a, &b);
            if (strcmp(result[2], "overflow") != 0)
                same(C_MUL, &x, result[2]);
            int order = ibz_cmp(&a, &b);
            same_int(C_CMP, (order > 0) - (order < 0), result[3]);
        } else if (strcmp(op, "unary") == 0) {
            // result: negation, sign, zero, odd, bit length, trailing
            // zeros, then (low bits, residue) for counts 0, 1, 2, 3, 63, 64.
            same_int(C_IS_ODD, ibz_is_odd(&a) ? 1 : 0, result[3]);
            if (!ibz_is_zero(&a))
                same_int(C_BITSIZE, ibz_bitsize(&a), result[4]);
            // The residues modulo 2, 4 and 8, as the reference's callers
            // read them: ibz_get(x) & 1, & 3, & 7.
            int32_t low = ibz_get(&a);
            same_int(C_GET, low & 1, result[6 + 2 * 1 + 1]);
            same_int(C_GET, low & 3, result[6 + 2 * 2 + 1]);
            same_int(C_GET, low & 7, result[6 + 2 * 3 + 1]);
        } else if (strcmp(op, "shift") == 0) {
            ibz_div_2exp(&x, &a, (uint32_t)strtoul(operand[1], NULL, 10));
            same(C_DIV_2EXP, &x, result[1]);
        } else if (strcmp(op, "div") == 0) {
            parse(&b, operand[1]);
            if (strcmp(result[0], "divzero") == 0)
                continue;
            ibz_div(&x, &y, &a, &b);
            same(C_DIV, &x, result[0]);
            same(C_DIV, &y, result[1]);
            ibz_div_floor(&x, &y, &a, &b);
            same(C_DIV_FLOOR, &x, result[2]);
            same(C_DIV_FLOOR, &y, result[3]);
            ibz_mod(&x, &a, &b);
            same(C_MOD, &x, result[4]);
            ibz_rounded_div(&x, &a, &b);
            same(C_ROUNDED_DIV, &x, result[5]);
            if (strcmp(result[6], "modulus") != 0) {
                ibz_centered_mod(&x, &a, &b);
                same(C_CENTERED_MOD, &x, result[6]);
            }
        } else if (strcmp(op, "xgcd") == 0) {
            parse(&b, operand[1]);
            ibz_gcd(&x, &a, &b);
            same(C_GCD, &x, result[0]);
        } else if (strcmp(op, "inv") == 0) {
            parse(&b, operand[1]);
            if (strcmp(result[0], "modulus") == 0)
                continue;
            int found = ibz_invmod(&x, &a, &b);
            compared[C_INVMOD]++;
            if (strcmp(result[0], "none") == 0) {
                if (found)
                    fail(C_INVMOD, "the reference has an inverse, the file has none");
            } else if (!found) {
                fail(C_INVMOD, "the reference has no inverse");
            } else {
                same(C_INVMOD, &x, result[0]);
            }
        } else if (strcmp(op, "sqrt") == 0) {
            if (strcmp(result[0], "negative") == 0)
                continue;
            ibz_sqrt_floor(&x, &a);
            same(C_SQRT_FLOOR, &x, result[0]);
            int square = ibz_sqrt(&y, &a);
            compared[C_SQRT]++;
            if (strcmp(result[1], "none") == 0) {
                if (square)
                    fail(C_SQRT, "the reference has a root, the file has none");
            } else if (!square) {
                fail(C_SQRT, "the reference has no root");
            } else {
                same(C_SQRT, &y, result[1]);
            }
        } else if (strcmp(op, "legendre") == 0) {
            parse(&b, operand[1]);
            same_int(C_LEGENDRE, ibz_legendre(&a, &b), result[0]);
        }
        (void)results;
    }
    free(line);
    fclose(file);
    ibz_finalize(&a);
    ibz_finalize(&b);
    ibz_finalize(&x);
    ibz_finalize(&y);
    // Every function named above was given something: a file that lost an
    // operation would otherwise pass by comparing nothing.
    long total = 0;
    for (int what = 0; what < C_COUNT; what++) {
        printf("%-18s %ld\n", NAMES[what], compared[what]);
        total += compared[what];
        if (compared[what] == 0) {
            fprintf(stderr, "%s was given nothing\n", NAMES[what]);
            failures++;
        }
    }
    printf("%ld comparisons, %ld differences\n", total, failures);
    return failures == 0 ? 0 : 1;
}
