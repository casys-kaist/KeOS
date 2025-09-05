#ifndef __LIB_SYSCALL_MMAN_H
#define __LIB_SYSCALL_MMAN_H

#define PROT_READ 0x1  /* Page can be read.  */
#define PROT_WRITE 0x2 /* Page can be written.  */
#define PROT_EXEC 0x4  /* Page can be executed.  */
#define PROT_NONE this_is_invalid_argument
// #define PROT_NONE        0x0                /* Page can not be accessed, we
// will never use this */

#endif /* lib/syscall-nr.h */