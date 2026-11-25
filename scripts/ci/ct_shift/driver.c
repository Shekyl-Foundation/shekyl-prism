// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

#include <stdint.h>
#include <stdlib.h>
#ifdef MEMCHECK
#include <valgrind/memcheck.h>
#endif

extern void prism_shift_probe(uint64_t input, uint64_t count);
static volatile uint64_t sink;

__attribute__((noinline)) void probe_end(void) { sink = 1; }

__attribute__((noinline)) void leaky_control(uint64_t bit) {
    if (bit & 1) {
        sink = 2;
    } else {
        sink = 3;
        sink = 4;
    }
}

int main(int argc, char **argv) {
    if (argc != 4) return 2;
    uint64_t x = strtoull(argv[2], NULL, 0);
    uint64_t k = strtoull(argv[3], NULL, 0);
#ifdef MEMCHECK
    VALGRIND_MAKE_MEM_UNDEFINED(&x, sizeof(x));
    VALGRIND_MAKE_MEM_UNDEFINED(&k, sizeof(k));
#endif
    if (argv[1][0] == 'c') leaky_control(k);
    else prism_shift_probe(x, k);
    probe_end();
    return 0;
}
