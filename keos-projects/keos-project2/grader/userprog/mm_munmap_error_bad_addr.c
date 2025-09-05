#include <debug.h>
#include <fcntl.h>
#include <mman.h>
#include <stdio.h>
#include <string.h>
#include <syscall.h>

int main(int argc, char *argv[]) {
  // NULL pointer munmap
  ASSERT(munmap(NULL) < 0);

  // Kernel address munmap
  ASSERT(munmap((void *)0xFFFFFF0000900000) < 0);

  printf("success ");
  return 0;
}
