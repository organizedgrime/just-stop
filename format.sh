#!/usr/bin/env bash

# Parse arguments
lossy=0
device="/dev/video0"
for arg in "$@"; do
  case $arg in
  --lossy) lossy=1 ;;
  *) device="$arg" ;;
  esac
done

# Get camera capabilities and parse with external awk script
v4l2-ctl --device="$device" --list-formats-ext 2>/dev/null |
  awk -v device="$device" -v prefer_lossy="$lossy" -f parse_camera.awk
