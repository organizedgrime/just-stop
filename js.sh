#!/usr/bin/env bash

# Parse arguments
VIRTUAL_MODE=0
WATCH_DIR=""
WEBCAM_DEV="/dev/video0"
VIRTUAL_DEV="/dev/video2"
FFMPEG_PID=""
INOTIFY_PID=""

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

# Get camera settings (dual setup for smooth preview + quality capture)
PREVIEW_ARGS=$(./format.sh --lossy "$WEBCAM_DEV")
CAPTURE_ARGS=$(./format.sh "$WEBCAM_DEV")
if [[ -z "$PREVIEW_ARGS" || -z "$CAPTURE_ARGS" ]]; then
  echo "Failed to get camera settings"
  exit 1
fi

eval "PREVIEW_ARGS_ARRAY=($PREVIEW_ARGS)"
eval "CAPTURE_ARGS_ARRAY=($CAPTURE_ARGS)"
RESOLUTION=$(echo "$PREVIEW_ARGS" | grep -o '[0-9]\+x[0-9]\+')

# Find next frame number
CAPTURE_COUNTER=1
LAST_FRAME=$(ls "$WATCH_DIR"/frame_*.jpg 2>/dev/null | tail -1)
if [[ -n "$LAST_FRAME" ]]; then
  LAST_NUM=$(basename "$LAST_FRAME" | grep -o '[0-9]\+')
  CAPTURE_COUNTER=$((LAST_NUM + 1))
fi

echo "Stop motion setup ready - next frame: frame_$(printf "%04d" $CAPTURE_COUNTER).jpg"
echo "📸 Use './capture.sh' to take photos"

# Start/restart ffmpeg with overlay
start_overlay() {
  local img="$1"
  [[ -n "$FFMPEG_PID" ]] && kill -TERM -$FFMPEG_PID 2>/dev/null

  if [[ -n "$img" ]]; then
    # Build common ffmpeg command (using smooth preview settings)
    local ffmpeg_cmd=(
      ffmpeg "${PREVIEW_ARGS_ARRAY[@]}"
      -loop 1 -i "$img"
      -filter_complex "[1:v]scale=$RESOLUTION,format=yuva420p,colorchannelmixer=aa=0.5[overlay];[0:v][overlay]overlay"
    )

    if [[ $VIRTUAL_MODE -eq 1 ]]; then
      # Virtual camera output
      "${ffmpeg_cmd[@]}" -pix_fmt yuyv422 -f v4l2 "$VIRTUAL_DEV" 2>/dev/null &
    else
      # Preview window output
      "${ffmpeg_cmd[@]}" -f nut -c:v rawvideo - 2>/dev/null |
        ffplay -loglevel error -vf setpts=0 -window_title "Overlay Preview" - 2>/dev/null &
    fi
    FFMPEG_PID=$!
  fi
}

# Photo capture function
capture_photo() {
  local filename="frame_$(printf "%04d" $CAPTURE_COUNTER).jpg"
  local filepath="$WATCH_DIR/$filename"

  echo "📸 $filename"
  ffmpeg "${CAPTURE_ARGS_ARRAY[@]}" -frames:v 1 -y "$filepath" -loglevel error 2>/dev/null

  if [[ $? -eq 0 ]]; then
    ((CAPTURE_COUNTER++))
  else
    echo "❌ Capture failed"
  fi
}

# Cleanup function
cleanup() {
  [[ "$CLEANUP_DONE" == "1" ]] && return
  CLEANUP_DONE=1

  echo -e "\nStopping..."
  [[ -n "$FFMPEG_PID" ]] && kill -KILL -$FFMPEG_PID 2>/dev/null
  [[ -n "$INOTIFY_PID" ]] && kill -KILL "$INOTIFY_PID" 2>/dev/null
  exit 0
}

# Set up signal handlers
trap 'capture_photo' USR1
trap cleanup EXIT INT TERM

# Start with latest existing image
latest_image=$(ls "$WATCH_DIR"/*.{jpg,png} 2>/dev/null | tail -1)
[[ -n "$latest_image" ]] && start_overlay "$latest_image"

# Monitor for new images
inotifywait -m -e create,modify,moved_to "$WATCH_DIR" --format '%w%f' 2>/dev/null | while read file; do
  [[ "$file" =~ \.(jpg|jpeg|png)$ ]] && start_overlay "$file"
done &
INOTIFY_PID=$!

# Monitor ffmpeg and exit when preview closes
while true; do
  if [[ -n "$FFMPEG_PID" ]] && ! kill -0 "$FFMPEG_PID" 2>/dev/null; then
    echo "Preview closed - exiting"
    exit 0
  fi
  sleep 1
done
