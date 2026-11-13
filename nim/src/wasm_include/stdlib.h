#ifndef ALKALAB_WASM_STDLIB_H
#define ALKALAB_WASM_STDLIB_H

#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

void *malloc(size_t size);
void *calloc(size_t count, size_t size);
void *realloc(void *ptr, size_t size);
void free(void *ptr);
void exit(int status) __attribute__((noreturn));

#ifdef __cplusplus
}
#endif

#endif
