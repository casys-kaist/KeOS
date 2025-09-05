#include <stdio.h>
#include <syscall.h>
#include <syscall-nr.h>
#include <fcntl.h>
#include <debug.h>
#include <string.h>

#define TRUELY_ERROR(x) ((int64_t)x < 0 && (-(int64_t)x) < 0x100)

struct checker_map {
    uint64_t syscall_nr;
    int attack_pos;
    int mask_arguments[6];
};

void* get_random_kernel_addr() {
    uint64_t addr;
    getrandom(&addr, sizeof(uint64_t), 0);

    addr = 0xffff800000000000 | (addr >> 17) & 0xfffffffffffff000;
    return (void*)addr;
}

static struct checker_map checker_maps[] = {
    {
        .syscall_nr = SYS_OPEN,
        .attack_pos = 0,
        .mask_arguments = { -1, 0, 0, -2, -2, -2 },
    },
    {
        .syscall_nr = SYS_READ,
        .attack_pos = 1,
        .mask_arguments = { 3, -1, 1024, -2, -2, -2 },
    },
    {
        .syscall_nr = SYS_READ,
        .attack_pos = 1,
        .mask_arguments = { 3, -1, 0, -2, -2, -2 },
    },
    {
        .syscall_nr = SYS_WRITE,
        .attack_pos = 1,
        .mask_arguments = { 3, -1, 1024, -2, -2, -2 },
    },
    {
        .syscall_nr = SYS_PIPE,
        .attack_pos = 0,
        .mask_arguments = { -1, -2, -2, -2, -2, -2 },
    },
    {
        .syscall_nr = SYS_MMAP,
        .attack_pos = 0,
        .mask_arguments = { -1, 4096, 1, 0, 3, 4096 },
    },
    {
        .syscall_nr = SYS_MUNMAP,
        .attack_pos = 0,
        .mask_arguments = { -1, 4096, -2, -2, -2, -2 },
    }
};

bool checker(const struct checker_map cm, bool use_mask, bool null_ptr_test) {
    uint64_t args[6];
    for (int j = 0; j < 6; j++) {
        if (use_mask) {
            if (cm.mask_arguments[j] == -1) {
                args[j] = (uint64_t)get_random_kernel_addr();
            } else {
                args[j] = (uint64_t)cm.mask_arguments[j];
            }
        } else {
            args[j] = (uint64_t)get_random_kernel_addr();
        }
    }

    if (cm.attack_pos >= 0 && cm.attack_pos < 6) {
        args[cm.attack_pos] = null_ptr_test ? 0 : (uint64_t)get_random_kernel_addr();
    }

    /* printf("Syscall(NR=%d, %p, %p, %p, %p, %p, %p)\n", cm.syscall_nr,
        args[0], args[1], args[2],
        args[3], args[4], args[5]); */

    int64_t ret = syscall(cm.syscall_nr,
                          args[0], args[1], args[2],
                          args[3], args[4], args[5]);
    
    return TRUELY_ERROR(ret);
}

int main(int argc, char *argv[]) {
    int test_no;
    bool use_mask, null_ptr_test;

    ASSERT(open("hello", O_RDWR) >= 3);

    ASSERT(read(3, (void*)0xffffff0000100000, 0x100) < 0);
    ASSERT(write(3, (void*)0x1, 0x100) < 0);
    
    for (int i = 0; i < 0x100; i++) {
        test_no = (uint64_t)get_random_kernel_addr() % 7;
        use_mask = (uint64_t)get_random_kernel_addr() % 2;
        null_ptr_test = (uint64_t)get_random_kernel_addr() % 2;
        ASSERT(checker(checker_maps[test_no], use_mask, null_ptr_test));
    }

    printf("success ");
    return 0;
}
