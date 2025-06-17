#!/bin/bash
set -uo pipefail

LOCKFILE="/tmp/just_stop.lock"
TRIGGER_FILE="/tmp/just_stop.trigger"

# real devices
devices=($(v4l2-ctl --list-devices | ./cameras.awk -v virtual=0))
# virtual devices
virtual_devices=($(v4l2-ctl --list-devices | ./cameras.awk -v virtual=1))

if [[ ${#devices[@]} -eq 0 ]]; then
  echo "There are no webcams available"
  exit 1
fi

if [[ ${#virtual_devices[@]} -eq 0 ]]; then
  echo "There are no virtual cameras available"
  exit 1
fi

VIRTUAL_CAM="${virtual_devices[0]}"
WEBCAM="${devices[0]}"
rows=0
columns=0
color="0xFF0000"
direction=
onion="0.25"
grid_opacity="0.5"
prefix="photo"

usage() {
  echo "Usage: $0 [-w webcam] [-v virtualcam] [-r rows] [-c cols] [-d h|v|b] [-C 0xRRGGBB] [-A opacity] [DIRECTORY]" >&2
  echo "Use -h for help" >&2
  exit 1
}

# If no options were specified at all, just print usage
if [[ ! -v 1 ]]; then
  usage
fi

while getopts ":v:w:r:c:d:o:p:C:A:h" o; do
  case $o in
  w | v | r | c | o)
    if [[ ! $OPTARG =~ ^[0-9]+$ ]]; then
      echo "Error: -$o requires a number" >&2
      exit 1
    fi
    ;;&
  w)
    selected_wcam="/dev/video$OPTARG"
    if [[ " ${devices[@]} " =~ " $selected_wcam " ]]; then
      echo "$selected_wcam is a valid webcam"
      WEBCAM=$selected_wcam
    else
      echo "$selected_wcam isn't a valid webcam." >&2
      echo -e "\nValid webcams: ${devices[@]}"
      exit 1
    fi
    ;;
  v)
    selected_vcam="/dev/video$OPTARG"
    if [[ " ${virtual_devices[@]} " =~ " $selected_vcam " ]]; then
      echo "$selected_vcam is a valid virtual device"
      VIRTUAL_CAM=$selected_vcam
    else
      echo "$selected_wcam isn't a valid virtual device." >&2
      echo -e "\nValid virtual devices: ${virtual_devices[@]}"
      exit 1
    fi
    ;;
  r | c | A | o)
    if [[ $OPTARG -lt 0 || $OPTARG -gt 100 ]]; then
      echo "Error: -$o must be >0 and <100" >&2
      exit 1
    fi
    ;;&
  r)
    rows=$OPTARG
    ;;
  c)
    columns=$OPTARG
    ;;
  d)
    if [[ ! $OPTARG =~ ^h|v|b$ ]]; then
      echo "Error: -$o must either be h, v, or b" >&2
      exit 1
    fi
    direction=$OPTARG
    ;;
  o)
    if [[ $OPTARG -eq 100 ]]; then
      onion="1.0"
    else
      onion="0.$OPTARG"
    fi
    ;;
  C)
    if [[ ! $OPTARG =~ ^0x(([0-9]|[A-F]){2}){3}$ ]]; then
      echo "Error: -$o must be a hexadecimal value in the form 0xRRGGBB" >&2
      exit 1
    fi
    color=$OPTARG
    ;;
  A)
    if [[ $OPTARG -eq 100 ]]; then
      grid_opacity="1.0"
    else
      grid_opacity="0.$OPTARG"
    fi
    ;;
  p)
    if [[ ! $OPTARG =~ ^[a-z]+$ ]]; then
      echo "Error: -$o must be lowercase alphabetical" >&2
      exit 1
    fi
    prefix=$OPTARG
    ;;
  h)
    cat <<EOF
Usage: $0 [OPTIONS] [DIRECTORY]

Video devices:
  -w <N>            Source webcam number (default: ${devices[0]##*/video})
  -v <N>            Virtual camera number (default: ${virtual_devices[0]##*/video})

Video effects:
  -d <h|v|b>        Direction: flip video feed horizontal, vertical, both

Grid:
  -r <0-100>        Rows (default: $rows)
  -c <0-100>        Columns (default: $columns)
  -C <0xRRGGBB>     Color (default: $color)
  -A <0-100>        Opacity % (default: $(echo "$grid_opacity * 100" | bc | cut -d. -f1))

File options:
  -p <S>            Prefix (default: $prefix)

Examples:
  $0 -w 0 -v 2 -r 16 -c 9 -d b -C 0x0000FF ~/Pictures
  $0 -r 5 -A 75 ~/Pictures/screenshots/
EOF
    exit 0
    ;;
  *)
    usage
    ;;
  esac
done
shift $((OPTIND - 1))

# Now that the vars have been shifted
if [[ ! -v 1 ]]; then
  echo "Error: Directory was not specified" >&2
  exit 1
fi

PHOTO_DIR="$1"

echo "Using webcam $WEBCAM and virtual output $VIRTUAL_CAM"

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
  local new_photo="$PHOTO_DIR/${prefix}_$timestamp.bmp"

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
    -vf "hflip,vflip" -frames:v 1 -y "$new_photo" 2>/dev/null; then
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

  #
  local webcam_filter=""
  if [[ -n "$direction" ]]; then
    case $direction in
    h)
      echo "Video will be flipped horizontally."
      webcam_filter+="hflip"
      ;;
    v)
      echo "Video will be flipped vertically."
      webcam_filter+="vflip"
      ;;
    b)
      echo "Video will be flipped both vertically and horizontally."
      webcam_filter+="hflip,vflip"
      ;;
    esac
  fi
  # Set to null if no settings were applied
  : ${webcam_filter:="null"}
  echo "this is the filter: $webcam_filter"

  if [[ $rows -gt 0 || $columns -gt 0 ]]; then
    if [[ -n webcam_filter ]]; then
      webcam_filter+=","
    fi
    webcam_filter+="drawgrid=w=iw/$columns:h=ih/$rows:t=4:c=$color@$grid_opacity"
    echo "Overlay grid will have $rows rows and $columns columns"
  fi

  # Start virtual camera with overlay
  ffmpeg -f v4l2 -input_format mjpeg -video_size 1920x1080 -framerate 30 -i "$WEBCAM" \
    -loop 1 -i "$PHOTO_DIR/latest.bmp" \
    -filter_complex "[0:v]$webcam_filter[webcam];[1:v]scale=1920x1080,format=yuva420p,colorchannelmixer=aa=${onion}[overlay];[webcam][overlay]overlay=0:0:format=auto,format=yuv420p" \
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
