#include <debug.h>
#include <fcntl.h>
#include <mman.h>
#include <stdio.h>
#include <string.h>
#include <syscall.h>

int main(int argc, char *argv[]) {
  int fd = open("hello", O_RDWR);
  ASSERT(fd >= 3);

  ASSERT(mmap((void *)0xA000, 0x3000, PROT_READ | PROT_WRITE, -1, 0) == (void*)0xA000);
  ASSERT(mmap((void *)0xD000, 0x1000, PROT_READ | PROT_WRITE, fd, 0) == (void*)0xD000);

  char* data = (char *) 0xD000;
  data[2] = ' ';

  (*((int *)0xAE00))++;
  (*((int *)0xBE00))++;
  (*((int *)0xCE00))++;

  ASSERT(memcmp((char *)0xD000, "We come to KeOS Project!", 24) == 0);
  ASSERT(read(fd, (void *)0xCFF8, 24) == 24);
  ASSERT(memcmp((char *)0xD000, "to KeOS Project!", 16) == 0);

  ASSERT(munmap((void *)0xA000) == 0);

  ASSERT(read(fd, (void *)0xAE00, 0x10) < 0);
  ASSERT(read(fd, (void *)0xBE00, 0x10) < 0);
  ASSERT(read(fd, (void *)0xCE00, 0x10) < 0);

  ASSERT(read(fd, (void *)0xDE00, 0x10) == 0x10);
  ASSERT(munmap((void *)0xD000) == 0);

  ASSERT(read(fd, (void *)0xDE00, 0x10) < 0);

  printf("success ");
  return 0;
}