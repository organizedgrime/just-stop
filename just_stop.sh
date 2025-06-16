#!/bin/bash

PHOTO_DIR="$1"
WEBCAM="/dev/video0"
VIRTUAL_CAM="/dev/video10"
PIDFILE="/tmp/just_stop.pid"
DIRFILE="/tmp/just_stop.folder"
echo "$PHOTO_DIR" >"$DIRFILE"
echo "saved $PHOTO_DIR to $DIRFILE"

# Cleanup function
cleanup() {
  echo -e "\nCleaning up..."
  [ -n "$FFMPEG_PID" ] && kill "$FFMPEG_PID" 2>/dev/null
  [ -f "$PIDFILE" ] && rm -f "$PIDFILE"
  [ -f "$DIRFILE" ] && rm -f "$DIRFILE"
  exit 0
}

# Set up signal handlers
trap cleanup SIGINT SIGTERM EXIT

# Create photo directory if it doesn't exist
if [[ ! -d $PHOTO_DIR ]]; then
  mkdir -p "$PHOTO_DIR"
fi

# Create placeholder image if no photos exist
if [ ! "$(ls -A "$PHOTO_DIR"/*.bmp 2>/dev/null)" ]; then
  bash ./capture_photo.sh && exit 0
  # ffmpeg -f lavfi -i "color=black:size=1920x1080:duration=0.1" -frames:v 1 -y "$PHOTO_DIR/placeholder.bmp" 2>/dev/null
fi

# Create/update symlink to latest photo
latest=$(find "$PHOTO_DIR" -name "*.bmp" -type f -printf '%T@ %p\n' 2>/dev/null | sort -nr | head -1 | cut -d' ' -f2-)
[ -n "$latest" ] && ln -sf "$latest" "$PHOTO_DIR/latest.bmp"

# Kill existing process
[ -f "$PIDFILE" ] && kill "$(cat "$PIDFILE")" 2>/dev/null

# Start virtual camera with overlay
ffmpeg -f v4l2 -input_format mjpeg -video_size 1920x1080 -framerate 30 -i "$WEBCAM" \
  -loop 1 -i "$PHOTO_DIR/latest.bmp" \
  -filter_complex "[1:v]scale=1920x1080,format=yuva420p,colorchannelmixer=aa=0.5[overlay];[0:v][overlay]overlay=0:0:format=auto,format=yuv420p" \
  -f v4l2 "$VIRTUAL_CAM" \
  2>/dev/null &

FFMPEG_PID=$!

echo "$FFMPEG_PID" >"$PIDFILE"
echo "saved $FFMPEG_PID to $PIDFILE"
echo "Virtual camera started (PID: $FFMPEG_PID)"
echo "Photo directory: $PHOTO_DIR"
echo "Press Ctrl+C to stop"

# Wait for processes
wait
