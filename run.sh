#!/usr/bin/env sh
if [ "$1" = "debug" ]; then
  qemu-system-x86_64 -S -s -drive format=raw,file=ossi.bin
else
  qemu-system-x86_64 -drive format=raw,file=ossi.bin
fi
