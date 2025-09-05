#include <stdint.h>

void set_cr0(uint64_t value) {
	__asm("mov %0, %%cr0" : : "r" (value));
}

int main(int argc, char *argv[]) {
	// If we are really in user mode, the following
	// instruction(s) should result in TRAP.
	set_cr0(0);

	return 1; // This should NOT be executed
}
