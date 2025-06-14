#!/usr/bin/env bash

# Setup tmp files
source tmp.sh

# Write this PID to file
write_tmp $$ host.pid

# Record
# export WEBCAM_DEV="/dev/video0"
# export VIRTUAL_DEV="/dev/video2"

write_tmp 0 camera.virtual
write_tmp 0 frames.count
write_tmp "/dev/video0" camera.device
write_tmp "frame" frames.prefix
write_tmp "1920x1080" camera.resolution

while [[ $# -gt 0 ]]; do
  case $1 in
  --virtual) write_tmp 1 camera.virtual ;;
  *) write_tmp $1 frames.dir ;;
  esac
  shift
done

frames_dir=$(read_tmp frames.dir)
if [[ -z $frames_dir ]]; then
  echo "Usage: $0 [--virtual] <watch_directory>"
  exit 1
else
  echo "frames.dir: $frames_dir"
  echo "camera.device: $(read_tmp camera.device)"
  echo "camera.virtual: $(read_tmp camera.virtual)"
fi

# Get camera settings (dual setup for smooth preview + quality capture)
source get_formats.sh
get_formats

# Sets up pause and resume handlers
source killer.sh
trap cleanup EXIT INT TERM

# Set up signal handlers
source overlay.sh

# Listen for USR1 signals
source capture_photo.sh
trap capture_photo USR1

# Start with latest existing image
prefix=$(read_tmp frames.prefix)
previous=$(ls "$frames_dir"/"$prefix"_*.bmp 2>/dev/null | tail -1)
count=$(basename "$previous" | grep -o '[0-9]\+')
if [[ -n "$count" ]]; then
  echo "Image tally is at $count."
  write_tmp $count frames.count
fi

echo "Stop motion setup ready."
echo "📸 Use './capture.sh' to take photos"

# Start by overlaying the previous image
if [[ -n "$previous" ]]; then
  overlay "$previous"
else
  overlay
fi

echo "started overlay, moving on to inotifywait"

# Monitor for new images
inotifywait -m -e create,modify,moved_to "$frames_dir" --format '%w%f' 2>/dev/null | while read file; do
  echo "noticed new file: $file"
  [[ "$file" =~ \.bmp$ ]] && overlay "$file"
  echo "tried to restart overlay"
done &
write_tmp $! inotify.pid
echo "inotify pid $(read_tmp inotify.pid)"

# # Monitor ffmpeg and exit when preview closes
while true; do
  sleep 1
done
