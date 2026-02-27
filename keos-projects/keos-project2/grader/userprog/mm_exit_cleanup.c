#include <debug.h>
#include <mman.h>
#include <stddef.h>
#include <stdint.h>
#include <syscall.h>

#define TEST_BASE ((void *)0x30000000)
#define TEST_SIZE (64 * 1024 * 1024)
#define PAGE_SIZE 4096

int main(int argc, char *argv[]) {
  uint8_t *buf = mmap(TEST_BASE, TEST_SIZE, PROT_READ | PROT_WRITE, -1, 0);
  ASSERT(buf == (uint8_t *)TEST_BASE);

  for (size_t off = 0; off < TEST_SIZE; off += PAGE_SIZE) {
    buf[off] = (uint8_t)(off / PAGE_SIZE);
  }

  return 0;
}
