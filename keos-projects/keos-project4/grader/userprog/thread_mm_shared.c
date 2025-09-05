#include <debug.h>
#include <mman.h>
#include <stdio.h>
#include <syscall.h>
#include <thread.h>

int prime_count = 0;

int thread_fn_1(void *arg) {
  int exitcode = -1;
  int thread_id = *(int *)arg;

  ASSERT(thread_join(thread_id, &exitcode) == 0);
  ASSERT(exitcode == 2);

  printf("Found %d primes\n", prime_count);
  exit(1);
  __builtin_unreachable();
}

int thread_fn_2(void *arg UNUSED) {
  int count = 0;
  for (int num = 2; num < 10000000; num++) {
    int prime = 1;
    for (int i = 2; i * i <= num; i++) {
      if (num % i == 0) {
        prime = 0;
        break;
      }
    }
    if (prime)
      count++;
  }

  prime_count = count;

  exit(2);
  __builtin_unreachable();
}

int main(int argc, char *argv[]) {
  void *stack_1 =
      mmap((void *)0xA000, STACK_SIZE, PROT_READ | PROT_WRITE, -1, 0);
  ASSERT(stack_1 == (void *)0xA000);

  void *stack_2 =
      mmap((void *)0xE000, STACK_SIZE, PROT_READ | PROT_WRITE, -1, 0);
  ASSERT(stack_2 == (void *)0xE000);

  int thread_id_2 =
      thread_create("my thread 2", stack_2 + STACK_SIZE, thread_fn_2, 0);
  ASSERT(thread_id_2 > 0);

  int thread_id_1 = thread_create("my thread 1", stack_1 + STACK_SIZE,
                                  thread_fn_1, &thread_id_2);
  ASSERT(thread_id_1 > 0);

  int exitcode = -1;
  ASSERT(thread_join(thread_id_1, &exitcode) == 0);
  ASSERT(exitcode == 1);

  return 0;
}
