#include <debug.h>
#include <fcntl.h>
#include <mman.h>
#include <stdio.h>
#include <string.h>
#include <syscall.h>

#define TRUELY_ERROR(x) ((int64_t)x < 0 && (-(int64_t)x) < 0x100)

int main(int argc, char *argv[]) {
  int i;

  // Invalid file descriptor
  ASSERT(TRUELY_ERROR(mmap((void *)0xA000UL, 0x1000, PROT_READ, 1337, 0)));
  ASSERT(write(1, (void *)0xA000UL, 0x1000) < 0);

  // STDIN, STDOUT, STDERR cannot be mmap-ed.
  for (i = 0; i < 3; i++) {
    ASSERT(TRUELY_ERROR(
        mmap((void *)(0xB000UL + i * 0x1000), 0x1000, PROT_READ, i, 0)));
    ASSERT(write(1, (char *)(0xB000UL + i * 0x1000), 0x1000) < 0);
  }

  printf("success ");
  return 0;
}
