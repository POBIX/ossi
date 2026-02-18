# ossi
A 32-bit x86 operating system written entirely from scratch in Rust. 

It's not meant for actual use, I made it mostly to satisfy my curiosity of how OSs work.

![ossi running in QEMU](https://i.imgur.com/kWylFyk.png)

# Features
- Memory protection: paging, an allocator, virtual memory
- Task scheduler: concurrently run multiple programs
- Userspace and kernel space separation, syscalls, GDT
- ELF program execution
- Hardware interfaces: VGA console, ATA, PS/2, I/O
- Interrupts: exceptions, timer, 8259 PIC, events
- Custom filesystem format
- A simple commandline interface
