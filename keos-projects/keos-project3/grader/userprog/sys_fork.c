#include <debug.h>
#include <fcntl.h>
#include <stdio.h>
#include <string.h>
#include <syscall.h>

int value = 0;

int main(int argc, char *argv[]) {
  int pid;
  int fds[2] = {0};
  char buf[1] = {0};

  ASSERT(pipe(fds) == 0);
  ASSERT(fds[0] >= 3);
  ASSERT(fds[1] >= 4);
  ASSERT(fds[1] > fds[0]);

  value = 1;
  pid = fork();
  ASSERT(pid == 0 || pid > 0);
  if (pid == 0) {
    ASSERT(value == 1);
    value = 2;
    printf("Hello, parent!\n");
    ASSERT(write(fds[1], "\0", 1) == 1);
  } else {
    ASSERT(read(fds[0], buf, 1) == 1);
    ASSERT(value == 1);
    printf("Hello, child!\n");
  }
  return 0;
}
