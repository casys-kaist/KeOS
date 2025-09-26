#include <stdio.h>
#include <stdint.h>
#include <string.h>
#include <fcntl.h>
#include <mman.h>
#include <syscall-nr.h>
#include <syscall.h>
#include <debug.h>

#define TRUELY_ERROR(x) ((int64_t)x < 0 && (-(int64_t)x) < 0x100)

const uint64_t always_zero = 0;
uint64_t elf_data = 0x31105;

uint64_t get_phys(void* addr, int mode) {
    return syscall2(SYS_GETPHYS, addr, mode);
}

int verify(void* addr, int mode) {
    int pid;
    uint64_t org_phys, new_phys, org_data;
    char dummy;
    int fds[2] = {0};

    printf("[CoW] Verify Virtual Address %p\n", addr);
    
    org_phys = get_phys(addr, 0);
    org_data = *(uint64_t *)addr;
    printf("[CoW] %p's PA = %llx, Data = %llx\n", addr, org_phys, org_data);
    ASSERT(!TRUELY_ERROR(org_phys));

    pipe(fds);
    pid = fork();
    ASSERT(pid >= 0);

    {
        printf("[CoW] %s: Before mutate, %p's PA = %llx, Data = %llx for me\n", pid ? "parent" : "child", addr, get_phys(addr, 0), *(uint64_t *)addr);
        /* Basic Sanity Check for both parent and child */
        ASSERT(get_phys(addr, 0) == org_phys);
        ASSERT(org_data == *(uint64_t *)addr);
    }

    // mode 0 => parent run first
    // mode 1 => child run first

    if ((pid == 0) == mode) {
        printf("[CoW] %s: Waiting for %s\n", pid ? "parent" : "child", pid ? "child" : "parent");
        while (read(fds[0], &dummy, 1) <= 0) {}

        printf("[CoW] %s: After mutate of %s, %p's PA = %llx, Data = %llx for me\n",
                pid ? "parent" : "child", pid ? "child" : "parent", addr, get_phys(addr, 0), *(uint64_t *)addr);
        ASSERT(get_phys(addr, 0) == org_phys);
        ASSERT(org_data == *(uint64_t *)addr);

        printf("[CoW] Still holds; pass for VA %p with mode %d\n", addr, mode);
    } else {
        printf("[CoW] %s: Mutate VA %p with random value\n", pid ? "parent" : "child", addr);
        getrandom(addr, sizeof(uint64_t), 0);

        printf("[CoW] %s: After mutate, %p's PA = %llx, Data = %llx for me\n", pid ? "parent" : "child", addr, get_phys(addr, 0), *(uint64_t *)addr);
        ASSERT(get_phys(addr, 0) != org_phys);
        ASSERT(org_data != *(uint64_t *)addr);

        printf("[CoW] %s: Signal %s for check intact\n", pid ? "parent" : "child", pid ? "child" : "parent");
        write(fds[1], &always_zero, 1);
    }

    if (pid == 0) {
        exit(0xc0ffee);
    } else {
        close(fds[0]);
        close(fds[1]);
    }
    return 1;
}

int main(int argc, char *argv[]) {
    volatile int dummy;
    int fd = open("hello", O_RDONLY);
    ASSERT(fd > 2);

    putchar('\n');

    dummy = always_zero & elf_data;     // Ensure that elf_data to be loaded into PT.
    verify(&elf_data, 0);
    verify(&elf_data, 1);

    void* anon = (void*)mmap((void*)0xA000UL, 0x1000, PROT_READ | PROT_WRITE, -1, 0);
    ASSERT(anon == (void*)0xA000UL);
    dummy = *((int*)anon);

    verify(anon, 0);
    verify(anon, 1);

    void* file = (void*)mmap((void*)0xB000UL, 0x1000, PROT_READ | PROT_WRITE, fd, 0);
    ASSERT(file == (void*)0xB000UL);
    dummy = *((int*)file);

    verify(file, 0);
    verify(file, 1);

	return 0;
}