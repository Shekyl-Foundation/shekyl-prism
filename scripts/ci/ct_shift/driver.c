// Copyright (c) 2025-2026, The Shekyl Foundation
//
// All rights reserved.
// BSD-3-Clause

#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#ifdef MEMCHECK
#include <valgrind/memcheck.h>
#endif

extern void prism_shift_probe(uint64_t input, uint64_t count);
struct encoded {
    unsigned char magnitude[168];
    unsigned char negative;
};
struct input {
    struct encoded norm, u[4], v[4];
};
extern void prism_reducer_probe(struct input basis, uint32_t guarded);
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
    if (argv[1][0] == 'r') {
        struct input basis;
        FILE *f = fopen(argv[2], "rb");
        if (!f || fread(&basis, sizeof(basis), 1, f) != 1) return 2;
        fclose(f);
#ifdef MEMCHECK
        VALGRIND_MAKE_MEM_UNDEFINED(&basis, sizeof(basis));
#endif
        prism_reducer_probe(basis, argv[3][0] == '1');
        return 0;
    }
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
