#include <debug.h>
#include <mman.h>
#include <stdio.h>
#include <syscall.h>
#include <thread.h>

int thread_fn(void *arg) {
  printf("Hello from thread!: %x\n", *(int *)arg);
  exit(0);
}

int main(int argc, char *argv[]) {
  int deadbeef = 0xdeadbeef;
  void *stack = mmap((void *)0xA000, STACK_SIZE, PROT_READ | PROT_WRITE, -1, 0);
  ASSERT(stack == 0xA000);

  int thread_id =
      thread_create("my thread", stack + STACK_SIZE, thread_fn, &deadbeef);
  ASSERT(thread_id > 0);

  int exitcode = -1;
  ASSERT(thread_join(thread_id, &exitcode) == 0);

  printf("Child thread exited with code %d\n", exitcode);

  return 0;
}
