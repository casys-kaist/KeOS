#include "include/syscall-nr.h"
#include <fcntl.h>
#include <syscall-nr.h>
#include <syscall.h>
#include <stddef.h>

void exit(int exitcode) {
  syscall1(SYS_EXIT, exitcode);
  __builtin_unreachable();
}

ssize_t open(const char *pathname, int flags) {
  return syscall2(SYS_OPEN, pathname, flags);
}

ssize_t read(int fd, const void *buf, size_t count) {
  return syscall3(SYS_READ, fd, buf, count);
}

ssize_t write(int fd, const void *buf, size_t count) {
  return syscall3(SYS_WRITE, fd, buf, count);
}

off_t seek(int fd, off_t offset, int whence) {
  return syscall3(SYS_SEEK, fd, offset, whence);
}

off_t tell(int fd) { return syscall1(SYS_TELL, fd); }

int close(int fd) { return syscall1(SYS_CLOSE, fd); }

int pipe(int pipefd[2]) { return syscall1(SYS_PIPE, pipefd); }

void *mmap(void *addr, size_t length, int prot, int fd, off_t offset) {
  return (void *)syscall5(SYS_MMAP, addr, length, prot, fd, offset);
}

int munmap(void *addr) { return syscall1(SYS_MUNMAP, addr); }
int fork() { return syscall0(SYS_FORK); }

int thread_create(const char *name, void *stack, int (*fn)(void *), void *arg) {
  return syscall4(SYS_THREAD_CREATE, name, stack, fn, arg);
}

int thread_join(int thread_id, int *exitcode) {
  return syscall2(SYS_THREAD_JOIN, thread_id, exitcode);
}

void exit_group(int exitcode) {
  syscall1(SYS_EXIT_GROUP, exitcode);
  __builtin_unreachable();
}
int create(char *name) { return syscall1(SYS_CREATE, name); }
int mkdir(char *name) { return syscall1(SYS_MKDIR, name); }
int unlink(char *name) { return syscall1(SYS_UNLINK, name); }
int chdir(char *name) { return syscall1(SYS_CHDIR, name); }
int readdir(int fd, struct dirent *dirents, int size) {
  return syscall3(SYS_READDIR, fd, dirents, size);
}
int stat(const char* pathname, struct stat *stat) {
  return syscall2(SYS_STAT, pathname, stat);
}
int fsync(int fd) { return syscall1(SYS_FSYNC, fd); }

/* "virtual" system call */
ssize_t getrandom(void *buf, size_t buflen, unsigned int flags) {
  if ((ssize_t)buflen < 0)
    return -22;

  size_t offset = 0;

  while (buflen - offset >= 8) {
    unsigned long rnd;
    __asm__ volatile("RDRAND %0" : "=a"(rnd));
    *(unsigned long *)((char *)buf + offset) = rnd;
    offset += 8;
  }

  size_t rem = buflen - offset;
  if (rem > 0) {
    unsigned long rnd;
    __asm__ volatile("RDRAND %0" : "=a"(rnd));
    unsigned char *src = (unsigned char *)&rnd;
    unsigned char *dst = (unsigned char *)buf + offset;
    for (size_t i = 0; i < rem; i++) {
      dst[i] = src[i];
    }
    offset = buflen;
  }

  return offset;
}