#!/usr/bin/env bash

# Parse arguments
VIRTUAL_MODE=0
WATCH_DIR=""
WEBCAM_DEV="/dev/video0"
VIRTUAL_DEV="/dev/video2"
FFMPEG_PID=""

for arg in "$@"; do
  case $arg in
  --virtual) VIRTUAL_MODE=1 ;;
  *) WATCH_DIR="$arg" ;;
  esac
done

if [[ -z "$WATCH_DIR" ]]; then
  echo "Usage: $0 [--virtual] <watch_directory>"
  exit 1
fi

# Get camera settings
CAMERA_ARGS=$(./format.sh --lossy "$WEBCAM_DEV")
CAPTURE_ARGS=$(./format.sh "$WEBCAM_DEV") # High quality for photos
if [[ -z "$CAMERA_ARGS" || -z "$CAPTURE_ARGS" ]]; then
  echo "Failed to get camera settings"
  exit 1
fi

# Convert to arrays
eval "CAMERA_ARGS_ARRAY=($CAMERA_ARGS)"
eval "CAPTURE_ARGS_ARRAY=($CAPTURE_ARGS)"

# Extract resolution
RESOLUTION=$(echo "$CAMERA_ARGS" | grep -o '[0-9]\+x[0-9]\+')

# Find next frame number
CAPTURE_COUNTER=1
if [[ -d "$WATCH_DIR" ]]; then
  EXISTING_FRAMES=($(ls "$WATCH_DIR"/frame_[0-9]*.jpg 2>/dev/null | sort))
  if [[ ${#EXISTING_FRAMES[@]} -gt 0 ]]; then
    LAST_FRAME=$(basename "${EXISTING_FRAMES[-1]}")
    LAST_NUM=$(echo "$LAST_FRAME" | grep -o '[0-9]\+' | tail -1)
    CAPTURE_COUNTER=$((LAST_NUM + 1))
  fi
fi

echo "Using settings: $CAMERA_ARGS"
echo "Resolution: $RESOLUTION"
echo "Next frame: frame_$(printf "%04d" $CAPTURE_COUNTER).jpg"
if [[ $VIRTUAL_MODE -eq 1 ]]; then
  echo "Virtual camera mode: output to $VIRTUAL_DEV"
else
  echo "Preview mode: showing window"
fi

# Start/restart ffmpeg with overlay
start_overlay() {
  local img="$1"
  [[ -n "$FFMPEG_PID" ]] && kill $FFMPEG_PID 2>/dev/null

  if [[ -n "$img" ]]; then
    echo "Starting overlay with: $(basename "$img")"
    if [[ $VIRTUAL_MODE -eq 1 ]]; then
      # Virtual camera output
      ffmpeg "${CAMERA_ARGS_ARRAY[@]}" \
        -loop 1 -i "$img" \
        -filter_complex "[1:v]scale=$RESOLUTION,format=yuva420p,colorchannelmixer=aa=0.5[overlay];[0:v][overlay]overlay" \
        -pix_fmt yuyv422 -f v4l2 "$VIRTUAL_DEV" &
    else
      # Preview window output
      ffmpeg "${CAMERA_ARGS_ARRAY[@]}" \
        -loop 1 -i "$img" \
        -filter_complex "[1:v]scale=$RESOLUTION,format=yuva420p,colorchannelmixer=aa=0.5[overlay];[0:v][overlay]overlay" \
        -f nut -c:v rawvideo - | ffplay -loglevel warning -vf setpts=0 -window_title "Overlay Preview" - &
    fi
    FFMPEG_PID=$!
  fi
}

# Photo capture function
capture_photo() {
  local filename="frame_$(printf "%04d" $CAPTURE_COUNTER).jpg"
  local filepath="$WATCH_DIR/$filename"

  echo "📸 Capturing: $filename"

  # Capture single frame with high quality
  ffmpeg "${CAPTURE_ARGS_ARRAY[@]}" -frames:v 1 -y "$filepath" -loglevel error 2>/dev/null

  if [[ $? -eq 0 ]]; then
    echo "✅ Saved: $filepath"
    ((CAPTURE_COUNTER++))
  else
    echo "❌ Failed to capture"
  fi
}

# Set up signal handler for photo capture
trap 'capture_photo' USR1

# Cleanup function
cleanup() {
  echo "Cleaning up..."
  [[ -n "$FFMPEG_PID" ]] && kill $FFMPEG_PID 2>/dev/null
  exit 0
}
trap cleanup EXIT INT TERM

echo "Monitoring $WATCH_DIR for changes..."
echo "📸 Use './capture.sh' to take photos"
echo "Press Ctrl+C to stop"

# Start with latest existing image
latest_image=$(find "$WATCH_DIR" -name "*.jpg" -o -name "*.png" 2>/dev/null | sort | tail -1)
if [[ -n "$latest_image" ]]; then
  start_overlay "$latest_image"
else
  echo "No existing images found - take your first photo!"
fi

# Monitor for new images
inotifywait -m -e create,modify,moved_to "$WATCH_DIR" --format '%w%f' 2>/dev/null | while read file; do
  if [[ "$file" =~ \.(jpg|jpeg|png)$ ]]; then
    echo "New image: $(basename "$file")"
    start_overlay "$file"
  fi
done &

# Keep script running
while true; do
  sleep 1
done
