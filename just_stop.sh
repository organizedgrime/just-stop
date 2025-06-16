#!/bin/bash

PHOTO_DIR="$1"
WEBCAM="/dev/video0"
VIRTUAL_CAM="/dev/video2"
PIDFILE="/tmp/just_stop.pid"

# Create photo directory if it doesn't exist
if [[ ! -d $PHOTO_DIR ]]; then
  mkdir -p "$PHOTO_DIR"
fi

# Cleanup function
cleanup() {
  echo "Cleaning up..."
  if [[ -f "$PIDFILE" ]]; then
    kill "$(cat "$PIDFILE")" 2>/dev/null
    rm -f "$PIDFILE"
  fi
  exit 0
}

capture() {
  local timestamp=$(date +"%Y_%m_%d_%H_%M_%S")
  local new_photo="$PHOTO_DIR/photo_$timestamp.bmp"

  # Kill virtual camera process
  if [ ! -f "$PIDFILE" ]; then
    echo "No PID to kill!"
  else
    PID=$(cat "$PIDFILE")
    rm -f "$PIDFILE"
    kill -TERM "$PID"
    while kill -0 $PID 2>/dev/null; do
      sleep 0.1
    done
    echo "Stopped virtual camera (PID: $PID)"
  fi

  # Capture photo
  ffmpeg -f v4l2 -input_format mjpeg -video_size 1920x1080 -i "$WEBCAM" -frames:v 1 -y "$NEW_PHOTO" 2>/dev/null
  CAPTURE_PID=$!
  while kill -0 $CAPTURE_PID 2>/dev/null; do
    sleep 0.1
  done

  echo "Captured: $NEW_PHOTO"
  echo "restarting preview..."
  preview
}

preview() {
  # Create/update symlink to latest photo
  local latest=$(find "$PHOTO_DIR" -name "*.bmp" -type f -printf '%T@ %p\n' 2>/dev/null | sort -nr | head -1 | cut -d' ' -f2-)
  [ -n "$latest" ] && ln -sf "$latest" "$PHOTO_DIR/latest.bmp"

  # Start virtual camera with overlay
  ffmpeg -f v4l2 -input_format mjpeg -video_size 1920x1080 -framerate 30 -i "$WEBCAM" \
    -loop 1 -i "$PHOTO_DIR/latest.bmp" \
    -filter_complex "[1:v]scale=1920x1080,format=yuva420p,colorchannelmixer=aa=0.5[overlay];[0:v][overlay]overlay=0:0:format=auto,format=yuv420p" \
    -f v4l2 "$VIRTUAL_CAM" \
    2>/dev/null &
  echo $! >"$PIDFILE"
  echo "Started virtual webcam with PID $(cat $PIDFILE)"
  wait
}

# Create placeholder image if no photos exist
if [ ! "$(ls -A "$PHOTO_DIR"/*.bmp 2>/dev/null)" ]; then
  capture
fi

# Cleanup when we actually get interrupted or terminated
trap cleanup SIGINT SIGTERM

# Capture when a USR1 signal is sent
trap capture USR1

preview
