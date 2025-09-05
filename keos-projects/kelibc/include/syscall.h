#ifndef __LIB_USER_SYSCALL_H
#define __LIB_USER_SYSCALL_H

#include <debug.h>
#include <fcntl.h>
#include <stdbool.h>
#include <stddef.h>
#include <stat.h>
#include <dirent.h>

__attribute__((always_inline)) static __inline int64_t
syscall(uint64_t num_, uint64_t a1_, uint64_t a2_, uint64_t a3_, uint64_t a4_,
        uint64_t a5_, uint64_t a6_) {
  int64_t ret;
  register uint64_t *num asm("rax") = (uint64_t *)num_;
  register uint64_t *a1 asm("rdi") = (uint64_t *)a1_;
  register uint64_t *a2 asm("rsi") = (uint64_t *)a2_;
  register uint64_t *a3 asm("rdx") = (uint64_t *)a3_;
  register uint64_t *a4 asm("r10") = (uint64_t *)a4_;
  register uint64_t *a5 asm("r8") = (uint64_t *)a5_;
  register uint64_t *a6 asm("r9") = (uint64_t *)a6_;

  __asm __volatile("mov %1, %%rax\n"
                   "mov %2, %%rdi\n"
                   "mov %3, %%rsi\n"
                   "mov %4, %%rdx\n"
                   "mov %5, %%r10\n"
                   "mov %6, %%r8\n"
                   "mov %7, %%r9\n"
                   "syscall\n"
                   : "=a"(ret)
                   : "g"(num), "g"(a1), "g"(a2), "g"(a3), "g"(a4), "g"(a5),
                     "g"(a6)
                   : "cc", "memory");
  return ret;
}

/* Invokes syscall NUMBER, passing no arguments, and returns the
   return value as an `int'. */
#define syscall0(NUMBER) (syscall(((uint64_t)NUMBER), 0, 0, 0, 0, 0, 0))

/* Invokes syscall NUMBER, passing argument ARG0, and returns the
   return value as an `int'. */
#define syscall1(NUMBER, ARG0)                                                 \
  (syscall(((uint64_t)NUMBER), ((uint64_t)ARG0), 0, 0, 0, 0, 0))
/* Invokes syscall NUMBER, passing arguments ARG0 and ARG1, and
   returns the return value as an `int'. */
#define syscall2(NUMBER, ARG0, ARG1)                                           \
  (syscall(((uint64_t)NUMBER), ((uint64_t)ARG0), ((uint64_t)ARG1), 0, 0, 0, 0))

#define syscall3(NUMBER, ARG0, ARG1, ARG2)                                     \
  (syscall(((uint64_t)NUMBER), ((uint64_t)ARG0), ((uint64_t)ARG1),             \
           ((uint64_t)ARG2), 0, 0, 0))

#define syscall4(NUMBER, ARG0, ARG1, ARG2, ARG3)                               \
  (syscall(((uint64_t)NUMBER), ((uint64_t)ARG0), ((uint64_t)ARG1),             \
           ((uint64_t)ARG2), ((uint64_t)ARG3), 0, 0))

#define syscall5(NUMBER, ARG0, ARG1, ARG2, ARG3, ARG4)                         \
  (syscall(((uint64_t)NUMBER), ((uint64_t)ARG0), ((uint64_t)ARG1),             \
           ((uint64_t)ARG2), ((uint64_t)ARG3), ((uint64_t)ARG4), 0))

#define syscall6(NUMBER, ARG0, ARG1, ARG2, ARG3, ARG4, ARG5)                   \
  (syscall(((uint64_t)NUMBER), ((uint64_t)ARG0), ((uint64_t)ARG1),             \
           ((uint64_t)ARG2), ((uint64_t)ARG3), ((uint64_t)ARG4),               \
           ((uint64_t)ARG5)))

void exit(int exitcode);
ssize_t open(const char *pathname, int flags);
ssize_t read(int fd, const void *buf, size_t count);
ssize_t write(int fd, const void *buf, size_t count);
off_t seek(int fd, off_t offset, int whence);
off_t tell(int fd);
int close(int fd);
int pipe(int pipefd[2]);
void *mmap(void *addr, size_t length, int prot, int fd, off_t offset);
int munmap(void *addr);
ssize_t getrandom(void *buf, size_t buflen, unsigned int flags);
int fork();
int thread_create(const char *name, void *stack, int (*fn)(void *), void *arg);
int thread_join(int thread_id, int *exitcode);
void exit_group(int exitcode);
int create(char *name);
int mkdir(char *name);
int unlink(char *name);
int chdir(char *name);
int readdir(int fd, struct dirent *dirents, int size);
int stat(const char* pathname, struct stat *stat);
int fsync(int fd);

#endif /* lib/user/syscall.h */