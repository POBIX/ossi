#!/usr/bin/env sh
com="qemu-system-x86_64"
args="-cdrom ossi.iso"
if [ "$1" = "debug" ]; then
  "$com" -S -s $args
else
  "$com" $args
fi
