/* Build against one PRISM level and print the public curve and its hint.
 *
 * The last public-key byte is the C hint. The bytes before it are A.
 * Linked with prism_lvl1, prism_lvl3, or prism_lvl5. Not part of the
 * Rust crates.
 */
#include <stdio.h>
#include <stdlib.h>

/* mini-gmp does not provide this. The reference calls it only to report a
 * failure inside ideal-to-isogeny. */
int gmp_printf(const char *fmt, ...) {
    (void)fmt;
    return 0;
}

#include "api.h"
#include "sig.h"

int main(void) {
    unsigned char *pk = calloc(CRYPTO_PUBLICKEYBYTES, 1);
    unsigned char *sk = calloc(CRYPTO_SECRETKEYBYTES, 1);
    if (pk == NULL || sk == NULL) {
        return 1;
    }
    if (prism_keypair(pk, sk) != 0) {
        return 1;
    }
    for (int i = 0; i < CRYPTO_PUBLICKEYBYTES - 1; i++) {
        printf("%02x", pk[i]);
    }
    printf(" %u\n", pk[CRYPTO_PUBLICKEYBYTES - 1]);
    return 0;
}
