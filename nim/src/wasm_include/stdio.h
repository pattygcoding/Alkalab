/* Minimal stdio.h for wasm32-unknown-unknown.
   The engine never prints; this only satisfies Nim's generated includes. */
#ifndef ALKALAB_WASM_STDIO_H
#define ALKALAB_WASM_STDIO_H

#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct FILE FILE;

extern FILE *stdout;
extern FILE *stderr;

size_t fwrite(const void *ptr, size_t size, size_t nmemb, FILE *stream);
int fputc(int c, FILE *stream);
int fflush(FILE *stream);

#ifdef __cplusplus
}
#endif

#endif
