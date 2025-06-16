#!/bin/bash
set -uo pipefail

PHOTO_DIR="$1"
WEBCAM="/dev/video0"
VIRTUAL_CAM="/dev/video2"
LOCKFILE="/tmp/just_stop.lock"
TRIGGER_FILE="/tmp/just_stop.trigger"

# Create photo directory if it doesn't exist
mkdir -p "$PHOTO_DIR"

# Exclusive instance lock using directory
if ! mkdir "$LOCKFILE" 2>/dev/null; then
  echo "Another instance is already running"
  exit 1
fi

# State management
FFMPEG_PID=""

# Cleanup function
cleanup() {
  echo "Cleaning up..."
  if [[ -n "$FFMPEG_PID" ]] && kill -0 "$FFMPEG_PID" 2>/dev/null; then
    kill -INT "$FFMPEG_PID" 2>/dev/null || true
    wait "$FFMPEG_PID" 2>/dev/null || true
  fi
  rm -f "$TRIGGER_FILE"
  rmdir "$LOCKFILE" 2>/dev/null || true
  exit 0
}

# Signal handler for photo capture
handle_capture() {
  CAPTURE_REQUESTED=1
}

capture() {
  local timestamp=$(date +"%Y_%m_%d_%H_%M_%S")
  local new_photo="$PHOTO_DIR/photo_$timestamp.bmp"

  echo "Capturing photo..."

  # Stop streaming
  if [[ -n "$FFMPEG_PID" ]] && kill -0 "$FFMPEG_PID" 2>/dev/null; then
    # Send interrupt signal to ffpmeg
    kill -INT "$FFMPEG_PID" 2>/dev/null || true
    # Wait for process to finish dying
    while kill -0 "$FFMPEG_PID" 2>/dev/null; do
      sleep 0.1
    done
    FFMPEG_PID=""
  fi

  # Capture photo with error handling
  if ffmpeg -f v4l2 -input_format mjpeg -video_size 1920x1080 -i "$WEBCAM" \
    -frames:v 1 -y "$new_photo" 2>/dev/null; then
    echo "Captured: $new_photo"
  else
    echo "Failed to capture photo, continuing..."
  fi

  echo "Restarting preview..."
  preview
}

preview() {
  # Create/update symlink to latest photo
  local latest=$(find "$PHOTO_DIR" -name "*.bmp" -type f -printf '%T@ %p\n' 2>/dev/null |
    sort -nr | head -1 | cut -d' ' -f2-)
  [[ -n "$latest" ]] && ln -sf "$latest" "$PHOTO_DIR/latest.bmp"

  # Start virtual camera with overlay
  ffmpeg -f v4l2 -input_format mjpeg -video_size 1920x1080 -framerate 30 -i "$WEBCAM" \
    -loop 1 -i "$PHOTO_DIR/latest.bmp" \
    -filter_complex "[1:v]scale=1920x1080,format=yuva420p,colorchannelmixer=aa=0.5[overlay];[0:v][overlay]overlay=0:0:format=auto,format=yuv420p" \
    -f v4l2 "$VIRTUAL_CAM" 2>/dev/null &

  FFMPEG_PID=$!

  # Check if process actually started
  sleep 0.5
  if kill -0 "$FFMPEG_PID" 2>/dev/null; then
    echo "Started virtual webcam with PID $FFMPEG_PID"
  else
    echo "Failed to start preview, retrying in 2 seconds..."
    FFMPEG_PID=""
    sleep 2
    preview
  fi
}

# Create placeholder image if no photos exist
if [[ ! "$(ls -A "$PHOTO_DIR"/*.bmp 2>/dev/null)" ]]; then
  capture
fi

# Setup signal handlers - only for cleanup
trap cleanup SIGINT SIGTERM EXIT

# Start preview
preview

# Main loop - check for trigger file
while true; do
  # Handle photo capture requests via trigger file
  if [[ -f "$TRIGGER_FILE" ]]; then
    rm -f "$TRIGGER_FILE"
    capture
  fi

  # Check if ffmpeg is still running, restart if needed
  if [[ -n "$FFMPEG_PID" ]] && ! kill -0 "$FFMPEG_PID" 2>/dev/null; then
    echo "FFmpeg process died, restarting..."
    FFMPEG_PID=""
    sleep 1
    preview
  fi

  sleep 0.5
done
