#include <stdio.h>
#include <stdint.h>
#include <string.h>
#include <fcntl.h>
#include <mman.h>
#include <syscall-nr.h>
#include <syscall.h>
#include <debug.h>

#define TRUELY_ERROR(x) ((int64_t)x < 0 && (-(int64_t)x) < 0x100)

#define PTE_P   (1UL << 0)   // Present
#define PTE_RW  (1UL << 1)   // Read/Write
#define PTE_US  (1UL << 2)   // User/Supervisor
#define PTE_PWT (1UL << 3)   // Page Write-Through
#define PTE_PCD (1UL << 4)   // Page Cache Disable
#define PTE_A   (1UL << 5)   // Accessed
#define PTE_D   (1UL << 6)   // Dirty
#define PTE_PAT (1UL << 7)   // PAT
#define PTE_G   (1UL << 8)   // Global
#define PTE_XD  (1UL << 63)  // Execute Disable (No-Execute)

const uint64_t always_zero = 0;
uint64_t elf_data = 0x31105;

uint64_t get_phys(void* addr, int mode) {
    return syscall2(SYS_GETPHYS, addr, mode);
}

int rw_verify(void* addr) {
    int pid;
    char dummy;
    int fds[2] = {0};
    uint64_t perm;
    
    perm = get_phys(addr, 1);
    ASSERT(!TRUELY_ERROR(perm));
    ASSERT(perm & PTE_RW);
    ASSERT(perm & PTE_XD);

    pipe(fds);
    pid = fork();
    ASSERT(pid >= 0);

    perm = get_phys(addr, 1);
    ASSERT(!TRUELY_ERROR(perm));
    ASSERT(!(perm & PTE_RW));
    ASSERT(perm & PTE_XD);

    if (pid == 0) {
        getrandom(addr, sizeof(uint64_t), 0);

        perm = get_phys(addr, 1);
        ASSERT(!TRUELY_ERROR(perm));
        ASSERT(perm & PTE_RW);
        ASSERT(perm & PTE_XD);
        
        write(fds[1], &always_zero, 1);
    } else {
        while (read(fds[0], &dummy, 1) <= 0) {}
    }
    
    if (pid == 0) {
        exit(0xc0ffee);
    } else {
        close(fds[0]);
        close(fds[1]);
    }
    printf("[CoW-perm]: R/W test pass for VA %p\n", addr);
    return 1;
}

int main(int argc, char *argv[]) {
    volatile int dummy;
    int pid;
    int fds[2] = {0};
    int fd = open("hello", O_RDONLY);
    ASSERT(fd > 2);

    putchar('\n');

    dummy = always_zero & elf_data;     // Ensure that elf_data to be loaded into PT.
    rw_verify(&elf_data);

    void* anon = (void*)mmap((void*)0xA000UL, 0x1000, PROT_READ | PROT_WRITE, -1, 0);
    ASSERT(anon == (void*)0xA000UL);
    dummy = *((int*)anon);

    rw_verify(anon);

    void* file = (void*)mmap((void*)0xB000UL, 0x1000, PROT_READ | PROT_WRITE, fd, 0);
    ASSERT(file == (void*)0xB000UL);
    dummy = *((int*)file);

    rw_verify(file);

    void* exec = (void*)mmap((void*)0xC000UL, 0x1000, PROT_READ | PROT_WRITE | PROT_EXEC, -1, 0);
    
    // 0:  48 31 c0                 xor    rax,rax
    // 3:  b0 42                    mov    al,0x42
    // 5:  c3                       ret
    ASSERT(memcpy(exec, "\x48\x31\xC0\xB0\x42\xC3", 6));

    pipe(fds);
    pid = fork();

    ASSERT((*(int (*)())exec)() == (int)0x42);

    if (pid == 0) {        
        ((char*)exec)[4] = 'A';
        ASSERT((*(int (*)())exec)() == (int)'A');

        write(fds[1], &always_zero, 1);
        exit(0x1337);
    } else {
        while (read(fds[0], &dummy, 1) <= 0) {}
        ASSERT((*(int (*)())exec)() == (int)0x42);
    }
    
    printf("[CoW-perm]: Executable page test pass ");

	return 0;
}