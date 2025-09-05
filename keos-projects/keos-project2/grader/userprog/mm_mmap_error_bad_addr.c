#include <debug.h>
#include <fcntl.h>
#include <mman.h>
#include <stdio.h>
#include <string.h>
#include <syscall.h>

#define TRUELY_ERROR(x) ((int64_t)x < 0 && (-(int64_t)x) < 0x100)

int main(int argc, char *argv[]) {
  // NULL pointer mmap
  ASSERT(TRUELY_ERROR(mmap(NULL, 0x1000, PROT_READ, -1, 0)));
  ASSERT(write(1, NULL, 0x1000) < 0);

  // Kernel address space mmap attempts
  ASSERT(
      TRUELY_ERROR(mmap((void *)0xFFFF8C0FFEE15000, 0x1000, PROT_READ, -1, 0)));
  ASSERT(write(1, (void *)0xFFFF8C0FFEE15000, 0x1000) < 0);

  ASSERT(
      TRUELY_ERROR(mmap((void *)0xDEADBEEFC5330000, 0x1000, PROT_READ, -1, 0)));
  ASSERT(write(1, (void *)0xDEADBEEFC5330000, 0x1000) < 0);

  // User space address that conflicts with executable
  ASSERT(TRUELY_ERROR(
      mmap((void *)0x400000, 0x2000, PROT_READ | PROT_WRITE, -1, 0)));
  ASSERT(read(0, (void *)0x400000, 0x1000) < 0);

  printf("success ");
  return 0;
}
