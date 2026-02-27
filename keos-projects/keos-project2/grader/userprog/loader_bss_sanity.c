#include <stdint.h>
#include <stdio.h>

#define PAGE_UP(x) (((size_t)(x) + 0xfff) & ~0xfff)

int uninit;
uint8_t huge_uninit[0x1200];

int main(int argc, char *argv[]) {
  (void)argc;
  (void)argv;

  if (&uninit >= (int *)&huge_uninit[0]) {
    return -1;
  }

  uint8_t *begin = (uint8_t *)&uninit;
  uint8_t *end = (uint8_t *)PAGE_UP(begin);
  for (uint8_t *addr = begin; addr < end; addr++) {
    if (*addr != 0) {
      return -1;
    }
  }

  begin = (uint8_t *)&huge_uninit[0x1000];
  end = (uint8_t *)PAGE_UP(begin);
  for (uint8_t *addr = begin; addr < end; addr++) {
    if (*addr != 0) {
      return -1;
    }
  }

  printf("success ");
  return 0;
}
