#include <stddef.h>
#include <stdint.h>
#include <syscall.h>

int main(int, char *[]);
void _start(int argc, char *argv[]);

void _start(int argc, char *argv[]) {
#ifdef THREADING
  exit_group(main(argc, argv));
#else
  exit(main(argc, argv));
#endif
}