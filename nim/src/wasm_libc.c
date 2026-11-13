/* Minimal freestanding libc for the wasm32-unknown-unknown build.
   Only the four functions Nim's generated C actually references are provided:
   memcpy, memset, strlen and exit. The standalone (bump) allocator in Nim's
   runtime means malloc/free are never called. */

typedef unsigned int alk_size_t;

void *memcpy(void *dst, const void *src, alk_size_t n);
void *memset(void *dst, int value, alk_size_t n);

extern unsigned char __heap_base;
static unsigned char *alk_heap = 0;

static void alk_ensure(alk_size_t need) {
  if (alk_heap == 0) alk_heap = &__heap_base;
  alk_size_t end = (alk_size_t)alk_heap + need;
  alk_size_t pages = (end + 65535u) / 65536u;
  alk_size_t have = __builtin_wasm_memory_size(0);
  if (pages > have) {
    __builtin_wasm_memory_grow(0, pages - have);
  }
}

void *malloc(alk_size_t size) {
  alk_size_t total = size + 16u;
  alk_ensure(total);
  alk_size_t p = ((alk_size_t)alk_heap + 15u) & ~15u;
  *(alk_size_t *)p = size;
  alk_heap = (unsigned char *)(p + total);
  return (void *)(p + 16u);
}

void free(void *ptr) {
  (void)ptr;
}

void *calloc(alk_size_t count, alk_size_t size) {
  alk_size_t total = count * size;
  void *p = malloc(total);
  memset(p, 0, total);
  return p;
}

void *realloc(void *ptr, alk_size_t size) {
  if (ptr == 0) return malloc(size);
  alk_size_t old_size = *(alk_size_t *)((unsigned char *)ptr - 16u);
  void *fresh = malloc(size);
  alk_size_t copy = old_size < size ? old_size : size;
  memcpy(fresh, ptr, copy);
  return fresh;
}

void *memcpy(void *dst, const void *src, alk_size_t n) {
  unsigned char *d = (unsigned char *)dst;
  const unsigned char *s = (const unsigned char *)src;
  alk_size_t i;
  for (i = 0; i < n; i++) d[i] = s[i];
  return dst;
}

void *memmove(void *dst, const void *src, alk_size_t n) {
  unsigned char *d = (unsigned char *)dst;
  const unsigned char *s = (const unsigned char *)src;
  if (d < s) {
    alk_size_t i;
    for (i = 0; i < n; i++) d[i] = s[i];
  } else if (d > s) {
    alk_size_t i = n;
    while (i > 0) {
      i--;
      d[i] = s[i];
    }
  }
  return dst;
}

void *memset(void *dst, int value, alk_size_t n) {
  unsigned char *d = (unsigned char *)dst;
  alk_size_t i;
  for (i = 0; i < n; i++) d[i] = (unsigned char)value;
  return dst;
}

int memcmp(const void *a, const void *b, alk_size_t n) {
  const unsigned char *x = (const unsigned char *)a;
  const unsigned char *y = (const unsigned char *)b;
  alk_size_t i;
  for (i = 0; i < n; i++) {
    if (x[i] != y[i]) return (int)x[i] - (int)y[i];
  }
  return 0;
}

alk_size_t strlen(const char *s) {
  alk_size_t n = 0;
  while (s[n] != '\0') n++;
  return n;
}

__attribute__((noreturn)) void exit(int status) {
  (void)status;
  __builtin_unreachable();
}

typedef struct FILE FILE;

FILE *stdout = (FILE *)0;
FILE *stderr = (FILE *)0;

alk_size_t fwrite(const void *ptr, alk_size_t size, alk_size_t nmemb, FILE *stream) {
  (void)ptr;
  (void)size;
  (void)nmemb;
  (void)stream;
  return 0;
}

int fputc(int c, FILE *stream) {
  (void)stream;
  return c;
}

int fflush(FILE *stream) {
  (void)stream;
  return 0;
}

double sqrt(double x) {
  return __builtin_sqrt(x);
}

double fabs(double x) {
  return __builtin_fabs(x);
}
