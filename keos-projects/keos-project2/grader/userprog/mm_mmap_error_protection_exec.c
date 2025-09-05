#include <debug.h>
#include <fcntl.h>
#include <mman.h>
#include <stdio.h>
#include <string.h>
#include <syscall.h>

int main(int argc, char *argv[]) {
  // Unlike sys_mmap testcase, it tests W^X rule.
  // Please refer to sys_mmap testcase if you're interested in.
  ASSERT(mmap((void *)0xE000, 0x1000, PROT_READ | PROT_WRITE, -1, 0) ==
        (void *)0xE000);

  // 0:  48 31 c0                 xor    rax,rax
  // 3:  b0 42                    mov    al,0x42
  // 5:  c3                       ret
  ASSERT(memcpy((void *)0xE000, "\x48\x31\xC0\xB0\x42\xC3", 6));
  (*(int (*)())0xE000)();

  return 0x1337; // This should be NEVER executed.
}