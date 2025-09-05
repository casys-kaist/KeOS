#include <stdio.h>

int main(int argc, char *argv[]) {
  printf("argc: %d\n", argc);
  for (int i = 0; i < argc; i++) {
    printf("argv[%d] = %s (%p)\n", i, argv[i], argv[i]);
  }
  return 0;
}