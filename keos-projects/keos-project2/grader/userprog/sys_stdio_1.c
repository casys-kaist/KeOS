#include <debug.h>
#include <fcntl.h>
#include <stdio.h>
#include <string.h>
#include <syscall.h>

int main(int argc, char *argv[]) {
  int i;
  char buf[16] = {0};

  ASSERT(read(0, buf, 12) == 12);
  ASSERT(strcmp(buf, "KeOS is fun!") == 0);

  ASSERT(read(0, NULL, 12) < 0);
  ASSERT(write(0, buf, 12) < 0);

  for (i = 0; i < 12; i++)
    buf[i] = -1;

  ASSERT(read(0, buf, 8) == 0);

  for (i = 0; i < 12; i++)
    ASSERT(buf[i] == -1);

  printf("success ");
  return 0;
}