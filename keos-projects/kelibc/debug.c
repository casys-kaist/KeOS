#include <debug.h>
#include <stdarg.h>
#include <stdbool.h>
#include <stddef.h>
#include <stdio.h>
#include <string.h>

#include <debug.h>
#include <stdarg.h>
#include <stdbool.h>
#include <stdio.h>
#include <syscall.h>

/* Aborts the user program, printing the source file name, line
   number, and function name, plus a user-specific message. */
void debug_panic(const char *file, int line, const char *function,
                 const char *message, ...) {
  va_list args;

  printf("User process ABORT at %s:%d in %s(): ", file, line, function);

  va_start(args, message);
  vprintf(message, args);
  printf("\n");
  va_end(args);

  debug_backtrace();

  exit(1);
}

/* Prints the call stack, that is, a list of addresses, one in
   each of the functions we are nested within.  gdb or addr2line
   may be applied to kernel.o to translate these into file names,
   line numbers, and function names.  */
void debug_backtrace(void) {
  static bool explained;
  void **frame;

  printf("Call stack:");
  for (frame = __builtin_frame_address(0); frame != NULL && frame[0] != NULL;
       frame = frame[0])
    printf(" %p", frame[1]);
  printf("\n");

  if (!explained) {
    explained = true;
    printf("The `addr2line' program can make call stacks useful.\n"
           "Read \"Debugging a User Process\" chapter in the\n"
           "KeOS documentation for more information.\n");
  }
}