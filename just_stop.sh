#!/usr/bin/env bash

# export PREVIEWER_PID=$$
echo "$$" >"/tmp/just_stop_previewer.pid" # Backup method

# Record
export WEBCAM_DEV="/dev/video0"
export VIRTUAL_DEV="/dev/video2"
export WINDOW_TITLE="Onion"
export PREFIX="frame"

export VIRTUAL_MODE=0
export WATCH_DIR=""

while [[ $# -gt 0 ]]; do
  case $1 in
  --virtual) export VIRTUAL_MODE=1 ;;
  *) export WATCH_DIR=$1 ;;
  esac
  shift
done

if [[ -z "$WATCH_DIR" ]]; then
  echo "Usage: $0 [--virtual] <watch_directory>"
  exit 1
fi

# Get camera settings (dual setup for smooth preview + quality capture)
source get_formats.sh
echo "resolution: $RESOLUTION"

# Set up signal handlers
source overlay.sh

source capture_photo.sh
trap capture_photo USR1

source cleanup.sh
trap cleanup EXIT INT TERM

# Start with latest existing image
export PREVIOUS_IMG=$(ls "$WATCH_DIR"/"$PREFIX"_*.jpg 2>/dev/null | tail -1)
export PREVIOUS_COUNT=$(basename "$PREVIOUS_IMG" | grep -o '[0-9]\+')

echo "Stop motion setup ready."
[[ -n "$PREVIOUS_COUNT" ]] && echo "Image tally is at $PREVIOUS_COUNT."
echo "📸 Use './capture.sh' to take photos"

# Start by overlaying the previous image
[[ -n "$PREVIOUS_IMG" ]] && overlay "$PREVIOUS_IMG"
[[ -z "$PREVIOUS_IMG" ]] && overlay

# Monitor for new images
inotifywait -m -e create,modify,moved_to "$WATCH_DIR" --format '%w%f' 2>/dev/null | while read file; do
  echo "noticed new file: $file"
  [[ "$file" =~ \.(jpg|jpeg|png)$ ]] && overlay "$file"
done &
export INOTIFY_PID=$!

# Monitor ffmpeg and exit when preview closes
while true; do
  if [[ -n "$PREVIOUS_IMG" ]] && ! pgrep -f "ffmpeg.*$WEBCAM_DEV" >/dev/null; then
    echo "Preview closed - exiting"
    exit 0
  fi
  sleep 1
done
