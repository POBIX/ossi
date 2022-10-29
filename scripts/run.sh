#!/usr/bin/env sh
com="qemu-system-i386"
args="-cdrom ossi.iso"
if [ "$1" = "debug" ]; then
  screen -d -m "$com" -S -s $args
  sleep 1
else
  "$com" $args
fi
