/* Copyright (c) 2025-2026, The Shekyl Foundation
 *
 * All rights reserved.
 * BSD-3-Clause
 */

/* Build against one PRISM level and print the public curve and its hint.
 *
 * The last public-key byte is the C hint. The bytes before it are A.
 * Linked with prism_lvl1, prism_lvl3, or prism_lvl5. Not part of the
 * Rust crates.
 *
 * randombytes is defined here so the reference's OS reader is not linked.
 * The seed is the argument. The same seed reproduces the same key, which
 * is what lets CI diff testdata/.
 */
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

/* mini-gmp does not provide this. The reference calls it only to report a
 * failure inside ideal-to-isogeny. */
int gmp_printf(const char *fmt, ...) {
    (void)fmt;
    return 0;
}

#include "api.h"
#include "sig.h"

static uint64_t rng_state;

/* SplitMix64. Public-domain expansion of the seed; the vectors are public. */
static uint64_t splitmix64(void) {
    uint64_t z = (rng_state += 0x9E3779B97F4A7C15ULL);
    z = (z ^ (z >> 30)) * 0xBF58476D1CE4E5B9ULL;
    z = (z ^ (z >> 27)) * 0x94D049BB133111EBULL;
    return z ^ (z >> 31);
}

int randombytes(unsigned char *x, unsigned long long xlen) {
    unsigned long long filled = 0;
    while (filled < xlen) {
        uint64_t word = splitmix64();
        for (int byte = 0; byte < 8 && filled < xlen; byte++, filled++) {
            x[filled] = (unsigned char)(word & 0xffu);
            word >>= 8;
        }
    }
    return 0;
}

int randombytes_select(unsigned char *buf, unsigned long long n) {
    return randombytes(buf, n);
}

void randombytes_init(unsigned char *entropy_input,
                      unsigned char *personalization_string,
                      int security_strength) {
    (void)entropy_input;
    (void)personalization_string;
    (void)security_strength;
}

static int parse_seed(const char *text, uint64_t *out) {
    char *end = NULL;
    unsigned long long value;
    if (text == NULL || text[0] == '\0') {
        return 1;
    }
    value = strtoull(text, &end, 0);
    if (end == text || *end != '\0') {
        return 1;
    }
    *out = (uint64_t)value;
    return 0;
}

int main(int argc, char **argv) {
    unsigned char *pk;
    unsigned char *sk;
    if (argc != 2 || parse_seed(argv[1], &rng_state) != 0 || rng_state == 0) {
        fprintf(stderr, "usage: %s <nonzero-seed>\n", argv[0]);
        return 2;
    }
    pk = calloc(CRYPTO_PUBLICKEYBYTES, 1);
    sk = calloc(CRYPTO_SECRETKEYBYTES, 1);
    if (pk == NULL || sk == NULL) {
        return 1;
    }
    if (prism_keypair(pk, sk) != 0) {
        free(pk);
        free(sk);
        return 1;
    }
    for (int i = 0; i < CRYPTO_PUBLICKEYBYTES - 1; i++) {
        printf("%02x", pk[i]);
    }
    printf(" %u\n", pk[CRYPTO_PUBLICKEYBYTES - 1]);
    memset(sk, 0, CRYPTO_SECRETKEYBYTES);
    free(sk);
    free(pk);
    return 0;
}
