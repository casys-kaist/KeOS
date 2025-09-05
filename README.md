# KeOS: KAIST Educational Operating System

**KeOS** is an educational project designed to help students learn core operating system concepts by implementing a minimal yet functional kernel from the ground up.

**Note:** KeOS is an **individual project**. All work must be completed independently.


## Important Notes

* **Do not share or publish your code.**

  Public forks and redistribution of this repository are strictly prohibited by the project license.
  Any unauthorized disclosure, including public forks, will result in **severe penalties**, including **zero credit for all projects**.

* During grading, all files **except those explicitly whitelisted** for each project will be **replaced with the original versions**.

  Your implementation **must compile and pass all test cases without modifying restricted files**.
  Submissions that fail to meet these requirements may receive no credit for the affected components.

## Projects

KeOS consists of five sequential projects, each building on the last.
You will implement the concepts covered in the class on each project, reinforcing your understanding through hands-on development.

1. **System Call** – Interface between user applications and the kernel
2. **Memory Management** – Basic memory management and user-space execution
3. **Advanced Memory Management** – Implementation of advanced memory management
4. **Process Management** – Advanced multi-threaded and process control
5. **File System** – File system with Journaling

For detailed instructions and documentation, refer to the [KeOS Manual](https://casys-kaist.github.io/KeOS/keos).

## Getting Started
```bash
$ mkdir keos
$ cd keos
$ curl https://raw.githubusercontent.com/casys-kaist/KeOS/refs/heads/main/scripts/install.sh | sh
```

## Related Projects
- [KeV](https://github.com/casys-kaist/kev): KAIST educational Virtualization for `Special Topics in Computer Science <Virtualization> (CS492)`.
