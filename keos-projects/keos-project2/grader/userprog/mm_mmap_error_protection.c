#include <debug.h>
#include <fcntl.h>
#include <mman.h>
#include <stdio.h>
#include <string.h>
#include <syscall.h>

int main(int argc, char *argv[]) {
  ASSERT(mmap((void *)0xE000, 0x1000, PROT_READ, -1, 0) == (void *)0xE000);
  *((int*)0xE000) = 0x31105;

  return 0x1337; // This should be NEVER executed.
}