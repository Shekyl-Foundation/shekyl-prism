/* Copyright (c) 2025-2026, The Shekyl Foundation. All rights reserved. BSD-3-Clause */
#include <stddef.h>
#include <valgrind/memcheck.h>
void rlc_secret(void *p, size_t n) { VALGRIND_MAKE_MEM_UNDEFINED(p, n); }
void rlc_public(void *p, size_t n) { VALGRIND_MAKE_MEM_DEFINED(p, n); }
