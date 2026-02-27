# OS-projects

This repository contains KeOS (KAIST Educational Operating System) project and its virtualization extension KeV (KAIST Educational Virtualization).

## Important Notes

* **Do not share or publish your code.**

  Public forks and redistribution of this repository are strictly prohibited by the project license.
  Any unauthorized disclosure, including public forks, will result in **severe penalties**, including **zero credit for all projects**.

* During grading, all files **except those explicitly whitelisted** for each project will be **replaced with the original versions**.

  Your implementation **must compile and pass all test cases without modifying restricted files**.
  Submissions that fail to meet these requirements may receive no credit for the affected components.

## Requirements

* **Processor**
  * A minimum of a 4-core x86_64 intel processor with Broadwell microarchitecture or newer is required. The CPU must also have hardware virtualization enabled.
* **Memory** (RAM)
  * **Minimum**: 2 GiB or more
  * **Optimal**: 4 GiB
* **Host OS**
  * **Supported**: Ubuntu 24.04 LTS

----

## KeOS: KAIST Educational Operating System

**KeOS** is an educational project designed to help students learn core operating system concepts by implementing a minimal yet functional kernel from the ground up.

**Note:** KeOS is an **individual project**. All work must be completed independently.

### getting started

```bash
$ mkdir keos
$ cd keos
$ curl https://raw.githubusercontent.com/casys-kaist/keos/refs/heads/main/scripts/install-keos.sh | sh
```

### Projects

KeOS consists of five sequential projects, each building on the last.
You will implement the concepts covered in the class on each project, reinforcing your understanding through hands-on development.

1. **System Call** – Interface between user applications and the kernel
2. **Memory Management** – Basic memory management and user-space execution
3. **Advanced Memory Management** – Implementation of advanced memory management
4. **Process Management** – Advanced multi-threaded and process control
5. **File System** – File system with Journaling

For detailed instructions and documentation, refer to the [KeOS Manual](https://casys-kaist.github.io/KeOS/keos).

----

## KeV: KAIST Educational Virtualization

**KeV** is an educational project designed to help students learn core virtualization concepts by implementing a minimal yet functional type-2 hypervisor from the KeOS project.

**Note:** KeV is an **individual project**, except for last project. Projects except the last one must be completed independently.

### getting started

```bash
$ mkdir kev
$ cd kev
$ curl https://raw.githubusercontent.com/casys-kaist/keos/refs/heads/main/scripts/install-kev.sh | sh
```

### Projects

0. **KeOS**
1. **VMCS and VMExits**
2. **Hardware Virtualization**
3. **Interrupt and I/O Virtualization**
4. **Final Project**

For detailed instructions and documentation, refer to the [KeV Manual](https://casys-kaist.github.io/KeOS/kev).

#### Ignore
b7902a7412af5ddbe0da6399d1b89e0385d5c0bf5696ba54c35431eb98d5e37a

<!-- Release Version: v1.1.0 (2026-2-27) -->