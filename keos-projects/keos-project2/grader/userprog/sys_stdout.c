#include <debug.h>
#include <fcntl.h>
#include <stdio.h>
#include <string.h>
#include <syscall.h>

int main(int argc, char *argv[]) {
  char buf[20];
  ASSERT(read(1, buf, 12) < 0);

  ASSERT(write(1, "Hello, keos!", 12) == 12);

  ASSERT(write(1, "", 0) == 0);

  ASSERT(write(1, NULL, 12) < 0);

  printf("success ");
  return 0;
}