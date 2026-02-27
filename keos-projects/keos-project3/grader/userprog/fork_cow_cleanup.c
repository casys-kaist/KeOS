#include <debug.h>
#include <mman.h>
#include <stddef.h>
#include <stdint.h>
#include <syscall.h>

#define TEST_BASE ((void *)0x30000000)
#define TEST_SIZE (64 * 1024 * 1024)
#define PAGE_SIZE 4096

int main(int argc, char *argv[]) {
  int fds[2] = {0};
  uint8_t sync = 0;
  uint8_t *buf = mmap(TEST_BASE, TEST_SIZE, PROT_READ | PROT_WRITE, -1, 0);
  ASSERT(buf == (uint8_t *)TEST_BASE);
  ASSERT(pipe(fds) == 0);

  for (size_t off = 0; off < TEST_SIZE; off += PAGE_SIZE) {
    buf[off] = 0x5a;
  }

  int pid = fork();
  ASSERT(pid >= 0);

  if (pid == 0) {
    for (size_t off = 0; off < TEST_SIZE; off += PAGE_SIZE) {
      buf[off] ^= 0xff;
    }
    ASSERT(write(fds[1], &sync, 1) == 1);
    return 0;
  }

  ASSERT(read(fds[0], &sync, 1) == 1);
  for (size_t off = 0; off < TEST_SIZE; off += 64 * PAGE_SIZE) {
    ASSERT(buf[off] == 0x5a);
  }
  return 0;
}
