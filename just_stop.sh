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

# Virtual device
device_v="${virtual_devices[0]}"
# Webcam device
device_w="${devices[0]}"

# Grid rows
grid_r=0
# Grid columns
grid_c=0
# Grid opacity
grid_g="0.5"
# Grid color
grid_C="0xFF0000"

# Video flip direction
effect_d=
# Onion opacity
effect_o="0.25"

# File save prefix
file_p="photo"

# Advanced mode includes a vstack of previous frame and current feed
advanced=false

usage() {
  echo "Usage: $0 [-w webcam] [-v virtualcam] [-d h|v|b] [-o opacity] [-r rows] [-c cols] [-g opacity] [-C 0xRRGGBB] [-p prefix] [-a] [DIRECTORY]" >&2
  echo "Use -h for help" >&2
  exit 1
}

# If no options were specified at all, just print usage
if [[ ! -v 1 ]]; then
  usage
fi

while getopts ":v:w:r:c:d:g:o:p:C:ah" o; do
  opt_prefix=
  case $o in
  w | v)
    opt_prefix="device"
    ;;&
  r | c | g | C)
    opt_prefix="grid"
    ;;&
  p)
    opt_prefix="file"
    ;;&
  d | o)
    opt_prefix="effect"
    ;;&
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
      declare OPTARG=$selected_wcam
    else
      echo "$selected_wcam isn't a valid webcam." >&2
      echo -e "\nValid webcams: ${devices[@]}"
      exit 1
    fi
    ;;&
  v)
    selected_vcam="/dev/video$OPTARG"
    if [[ " ${virtual_devices[@]} " =~ " $selected_vcam " ]]; then
      echo "$selected_vcam is a valid virtual device"
      declare OPTARG=$selected_vcam
    else
      echo "$selected_wcam isn't a valid virtual device." >&2
      echo -e "\nValid virtual devices: ${virtual_devices[@]}"
      exit 1
    fi
    ;;&
  # Zero to one hundred values
  r | c | g | o)
    if [[ $OPTARG -lt 0 || $OPTARG -gt 100 ]]; then
      echo "Error: -$o must be >0 and <100" >&2
      exit 1
    fi
    ;;&
  # Direction
  d)
    if [[ ! $OPTARG =~ ^h|v|b$ ]]; then
      echo "Error: -$o must either be h, v, or b" >&2
      exit 1
    fi
    ;;&
  # Colors
  C)
    if [[ ! $OPTARG =~ ^0x(([0-9]|[A-F]){2}){3}$ ]]; then
      echo "Error: -$o must be a hexadecimal value in the form 0xRRGGBB" >&2
      exit 1
    fi
    ;;&
  # Percentages
  g | o)
    declare OPTARG=$(echo "scale=2; $OPTARG / 100" | bc)
    ;;&
  # Prefix
  p)
    if [[ ! $OPTARG =~ ^[a-z]+$ ]]; then
      echo "Error: -$o must be lowercase alphabetical" >&2
      exit 1
    fi
    ;;&
  # Set variable
  v | w | r | c | d | o | p | g | C)
    declare "${opt_prefix}_${o}=$OPTARG"
    ;;
  # Advanced mode
  a)
    advanced=true
    ;;
  h)
    cat <<EOF
Usage: $0 [OPTIONS] [DIRECTORY]

Video devices:
  -w <N>            Source webcam number (default: ${devices[0]##*/video})
  -v <N>            Virtual camera number (default: ${virtual_devices[0]##*/video})

Video effects:
  -d <h|v|b>        Direction: flip video feed horizontal, vertical, both
  -o <0-100>        Onion skin opacity % (default: $(echo "$effect_o * 100" | bc | cut -d. -f1))

Grid:
  -r <0-100>        Rows (default: $grid_r)
  -c <0-100>        Columns (default: $grid_c)
  -g <0-100>        Grid opacity % (default: $(echo "$grid_g * 100" | bc | cut -d. -f1))
  -C <0xRRGGBB>     Color (default: $grid_C)

Script options:
  -p <S>            File prefix (default: $file_p)
  -a                Turn on advanced mode

Examples:
  $0 -w 0 -v 2 -r 16 -c 9 -a -d b -C 0x0000FF ~/Pictures
  $0 -r 5 -g 75 ~/Pictures/screenshots/
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

echo "Using webcam $device_w and virtual output $device_v"

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
  local new_photo="$PHOTO_DIR/${file_p}_$timestamp.bmp"

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
  if ffmpeg -f v4l2 -input_format yuyv422 -video_size 1920x1080 -i "$device_w" \
    -vf "hflip,vflip" -frames:v 1 -framerate 5 -lossless 1 -y "$new_photo" 2>/dev/null; then
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
  local direction_filter=
  if [[ -n "$effect_d" ]]; then
    case $effect_d in
    h)
      echo "Video will be flipped horizontally."
      direction_filter+="hflip"
      ;;
    v)
      echo "Video will be flipped vertically."
      direction_filter+="vflip"
      ;;
    b)
      echo "Video will be flipped both vertically and horizontally."
      direction_filter+="hflip,vflip"
      ;;
    esac
  fi

  local grid_filter=
  if [[ $grid_r -gt 0 || $grid_c -gt 0 ]]; then
    grid_filter="drawgrid=w=iw/$grid_c:h=ih/$grid_r:t=4:c=$grid_C@$grid_g"
    echo "Overlay grid will have $grid_r rows and $grid_c columns"
  fi

  # Set to null if no settings were applied
  : ${direction_filter:="null"}
  : ${grid_filter:="null"}

  local filters=()

  local format_yuyv="format=yuv420p"
  local main_fmt="scale=1920x1080,${format_yuyv}"
  local thumb_fmt="scale=960x540,${format_yuyv}"
  local webcam_filter="${direction_filter},${grid_filter}"
  local onion_filter="blend=all_mode=normal:all_opacity=${effect_o}"

  # If we're in advanced mode
  if [[ $advanced = true ]]; then
    filters=(
      "[0:v]split=2[webcam][webcam_thumb]"
      "[1:v]split=2[latest][latest_thumb]"
      "[webcam_thumb]${thumb_fmt},${direction_filter}[webcam_thumb_filtered]"
      "[latest]${main_fmt}[overlay]"
      "[latest_thumb]${thumb_fmt}[latest_thumb_scaled]"
      "[webcam]${main_fmt},${webcam_filter}[webcam_filtered]"
      "[webcam_filtered][overlay]${onion_filter}[standard]"
      "[webcam_thumb_filtered][latest_thumb_scaled]vstack=inputs=2[left_stack]"
      "[left_stack][standard]hstack=inputs=2[output]"
    )
  else
    filters=(
      "[0:v]${main_fmt},${webcam_filter}[webcam]"
      "[1:v]${main_fmt}[latest]"
      "[webcam][latest]${onion_filter}[output]"
    )
  fi

  # Join filters into single string
  local filter_complex=$(
    IFS=\;
    echo "${filters[*]}"
  )

  # Start virtual camera with overlay
  ffmpeg -f v4l2 -input_format mjpeg -video_size 1920x1080 -framerate 30 -i "$device_w" \
    -loop 1 -i "$PHOTO_DIR/latest.bmp" \
    -filter_complex $filter_complex -map "[output]" \
    -f v4l2 "$device_v" 2>/dev/null &

  FFMPEG_PID=$!

  # Check if process actually started
  echo "Started virtual webcam with PID $FFMPEG_PID"
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
