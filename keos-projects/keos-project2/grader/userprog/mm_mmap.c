#include <debug.h>
#include <fcntl.h>
#include <mman.h>
#include <stdio.h>
#include <string.h>
#include <syscall.h>

int main(int argc, char *argv[]) {
  int fd = open("hello", O_RDWR);
  ASSERT(fd >= 3);

  ASSERT(mmap((void *)0xA000, 0x1000, PROT_READ | PROT_WRITE, -1, 0) == 0xA000);
  (*((int *)0xA000))++;

  ASSERT(mmap((void *)0xB000, 0x1000, PROT_READ, fd, 0) == 0xB000);
  ASSERT(memcmp((char *)0xB000, "Welcome to KeOS Project!", 24) == 0);

  ASSERT(read(fd, (void *)0xA000, 0x10) == 0x10);
  ASSERT(read(fd, (void *)0xB000, 0x10) < 0);

  /* For those who are interested at cybersecurity:
   *
   * In real programming, please abide at Write XOR eXecute (W^X) rule unless
   * you can guarantee the behaviour and immediately disallow the write. (e.g.,
   * JIT Compilation)
   *
   * The following code is making a memory map which allows both write and
   * execution (W&X), writing a small portion of binary code ("shellcode") and
   * executing it.
   *
   * If malicious actor can access the W&X memory area, and inject their
   * shellcode, it means that the actor can execute whatever they want.
   */
  ASSERT(mmap((void *)0xD000, 0x1000, PROT_READ | PROT_WRITE | PROT_EXEC, -1,
              0) == 0xD000);

  // 0:  48 31 c0                 xor    rax,rax
  // 3:  b0 42                    mov    al,0x42
  // 5:  c3                       ret
  ASSERT(memcpy((void *)0xD000, "\x48\x31\xC0\xB0\x42\xC3", 6));
  ASSERT((*(int (*)())0xD000)() == (int)0x42);

  printf("success ");
  return 0;
}
