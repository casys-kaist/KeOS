#include <debug.h>
#include <fcntl.h>
#include <mman.h>
#include <stdio.h>
#include <string.h>
#include <syscall.h>

int main(int argc, char *argv[]) {
  int fd = open("hello", O_RDWR);
  ASSERT(fd >= 3);

  ASSERT(mmap((void *)0xA000, 0x1000, PROT_READ | PROT_WRITE, -1, 0) == (void*)0xA000);
  (*((int *)0xA000))++;
  ASSERT(munmap((void *)0xA000) == 0);

  ASSERT(write(fd, (void *)0xA000, 0x10) < 0);

  ASSERT(mmap((void *)0xA000, 0x1000, PROT_READ, fd, 0) == (void*)0xA000);
  ASSERT(memcmp((char *)0xA000, "Welcome to KeOS Project!", 24) == 0);
  ASSERT(read(fd, (void *)0xA000, 0x10) < 0);
  ASSERT(munmap((void *)0xA000) == 0);

  ASSERT(write(fd, (void *)0xA000, 0x10) < 0);

  printf("success ");
  return 0;
}
