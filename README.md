# Overview

Bear is a VM and assembler.

## Example usage:

The following will assemble the file `roms/hello.bear`, load the resulting binary into the vm, and run it.
```bash
$ ./runner.sh roms/hello
```

# Quick Start

# VM

## Devices

Each device is identified by a non-negative integer.
The devices 0 - 16 are reserved.

- Device 0 is the "root device", it can be queried for information about the other attached devices.

# Assembler (bear-ass)
