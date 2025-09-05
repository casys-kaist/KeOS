#ifndef __LIB_STDDEF_H
#define __LIB_STDDEF_H

#include <stdint.h>

#define NULL ((void *) 0)
#define offsetof(TYPE, MEMBER) ((size_t) &((TYPE *) 0)->MEMBER)

typedef int64_t ssize_t;
typedef int64_t off_t;
typedef uint64_t time_t;
typedef uint32_t mode_t;

/* GCC predefines the types we need for ptrdiff_t and size_t,
 * so that we don't have to guess. */
typedef __PTRDIFF_TYPE__ ptrdiff_t;
typedef __SIZE_TYPE__ size_t;

#endif /* lib/stddef.h */