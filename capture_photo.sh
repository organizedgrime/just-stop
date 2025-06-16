#!/bin/bash
# capture_photo.sh - Capture photo and restart virtual camera

WEBCAM="/dev/video0"
PIDFILE="/tmp/just_stop.pid"
DIRFILE="/tmp/just_stop.folder"
TIMESTAMP=$(date +"%Y%m%d_%H%M%S")

# Setup for photo capture
PHOTO_DIR=$(cat "$DIRFILE")
if [[ ! -d $PHOTO_DIR ]]; then
  mkdir -p "$PHOTO_DIR"
fi
NEW_PHOTO="$PHOTO_DIR/photo_$TIMESTAMP.bmp"

# Kill virtual camera process
if [ -f "$PIDFILE" ]; then
  PID=$(cat "$PIDFILE")
  kill "$PID" 2>/dev/null
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

# Update latest symlink
ln -sf "$NEW_PHOTO" "$PHOTO_DIR/latest.bmp"

# Start virtual camera
bash ./just_stop.sh "$PHOTO_DIR"
