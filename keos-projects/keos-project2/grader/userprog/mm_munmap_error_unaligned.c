#include <debug.h>
#include <fcntl.h>
#include <mman.h>
#include <stdio.h>
#include <string.h>
#include <syscall.h>

int main(int argc, char *argv[]) {
  // Test unaligned/partial munmap
  ASSERT(mmap((void *)0xA000, 0x2000, PROT_READ, -1, 0) == (void *)0xA000);
  ASSERT(munmap((void *)0xB000) < 0);  // Unaligned/partial munmap should fail

  printf("success ");
  return 0;
}
