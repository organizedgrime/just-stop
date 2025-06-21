#!/bin/bash
set -uo pipefail

TMPDIR="/tmp/just_stop"
TRIGGER_CAPTURE="$TMPDIR/capture.trigger"
TRIGGER_DELETION="$TMPDIR/delete.trigger"
TRIGGER_PLAYBACK="$TMPDIR/playback.trigger"
PLAYBACK_FILE="$TMPDIR/playback.mp4"
NOTIFICATION_FILE="$TMPDIR/notification.txt"

# FFMPEG needs this arg in complex filters to work
CAMERA_FORMAT="format=yuv420p"

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
# File count
file_c=0
latest=""

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
      vcam_fps=$(v4l2-ctl -d2 -P | perl -n -e'/(\d+)\// && print $1')
      echo "vcam_fps: $vcam_fps"
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
SYMLINK="$PHOTO_DIR/latest.bmp"

echo "Using webcam $device_w and virtual output $device_v"

# Create photo directory if it doesn't exist
mkdir -p "$PHOTO_DIR"

# If the dir is still there but this is the only process
if [[ -d $TMPDIR && $(pgrep -c "just_stop.sh") -eq 1 ]]; then
  echo "Previous instance failed to clean up properly. Removing tmp files."
  rm -rf $TMPDIR
fi

# Exclusive instance lock using directory
if ! mkdir "$TMPDIR" 2>/dev/null; then
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
  rm -rf $TMPDIR 2>/dev/null
  exit 0
}

wait_for_pid() {
  if [[ -n "$1" ]]; then
    while kill -0 "$1" 2>/dev/null; do
      sleep 0.1
    done
  fi
}

stop_preview() {
  # Stop streaming
  if [[ -n "$FFMPEG_PID" ]] && kill -0 "$FFMPEG_PID" 2>/dev/null; then
    echo "Killing stream..."
    # Send interrupt signal to ffpmeg
    kill -INT "$FFMPEG_PID" 2>/dev/null || true
    # Wait for process to finish dying
    wait_for_pid "$FFMPEG_PID"
    # Reset pid
    FMPEG_PID=""
    echo "Stream is dead."
  fi
}

link_latest() {
  # Store all matching files with timestamps
  local file_list=$(find "$PHOTO_DIR" -name "*.bmp" -type f -printf '%T@ %p\n' 2>/dev/null)
  file_c=$(find "$PHOTO_DIR" -name "*.bmp" -type f | wc -l)
  # Count the number of matching files
  echo "file_c: ${file_c}"
  if [[ $file_c = 0 ]]; then
    echo "There are no pictures yet! Let me fix that"
    capture
    link_latest
  else
    # Get the latest file path
    latest=$(echo "$file_list" | sort -nr | head -1 | cut -d' ' -f2-)
    echo "latest is $latest"
    [[ -n "$latest" ]] && ln -sf "$latest" "$SYMLINK"
  fi
}

delete() {
  echo "Deleting Photo..." >"$NOTIFICATION_FILE"
  stop_preview

  local latest_referant=$(ls -l "$SYMLINK" | awk '/->/ {print $NF }')
  echo "Deleting $latest_referant"
  rm $latest_referant

  link_latest

  echo "Restarting preview"
  preview
}

capture() {
  echo "Capturing Photo..." >"$NOTIFICATION_FILE"

  stop_preview

  local timestamp=$(date +"%Y_%m_%d_%H_%M_%S")
  local new_photo="$PHOTO_DIR/${file_p}_${file_c}_$timestamp.bmp"
  echo "Capturing photo..."
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

playback() {
  echo "Forming video..."

  # Cleanup existing file if its already there
  if [[ -f "$PLAYBACK_FILE" ]]; then
    rm "$PLAYBACK_FILE"
  fi

  echo "Rendering Video..." >"$NOTIFICATION_FILE"

  local filters=()

  local main_fmt="scale=height=ih:width=iw,${CAMERA_FORMAT}"
  # Render in the webcam's native fps so it gets played back right
  if [[ $advanced = true ]]; then
    filters=(
      "[0:v]fps=${vcam_fps},${main_fmt}[format]"
      "[format]pad=width=iw*1.5:height=ih:x=(ow-iw):y=0:color=gray[output]"
    )
  else
    filters=(
      "[0:v]fps=${vcam_fps},${main_fmt}[output]"
    )
  fi

  # Join filters into single string
  local filter_complex=$(
    IFS=\;
    echo "${filters[*]}"
  )

  # Render a preview
  ffmpeg -framerate 12 -pattern_type glob -i "$PHOTO_DIR/$file_p*.bmp" -filter_complex "$filter_complex" -map "[output]" -c:v libx264 "$PLAYBACK_FILE" 2>/dev/null &

  wait_for_pid $!

  if [[ $? -eq 0 ]]; then
    stop_preview

    ffmpeg -re -i "$PLAYBACK_FILE" -f v4l2 "$device_v" 2>/dev/null &
    wait_for_pid $!

    echo "Restarting preview..."

    preview
  else
    echo "Failed to create mp4 from frames"
  fi
}

preview() {
  link_latest

  # Clear the notification before previewing
  echo "" >"$NOTIFICATION_FILE"

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

  local main_fmt="${CAMERA_FORMAT}"
  local thumb_fmt="scale=width=iw/2:height=ih/2,${CAMERA_FORMAT}"
  local webcam_filter="${direction_filter},${grid_filter}"
  local onion_filter="blend=all_mode=normal:all_opacity=${effect_o}"
  local file_count="drawtext=text='${file_p}_${file_c}':fontcolor=white:fontsize=30:box=1:boxcolor=black@${grid_g}"
  local notification_filter="drawtext=textfile=${NOTIFICATION_FILE}:reload=1:fontcolor=white:fontsize=100:box=1:boxcolor=black:x=(w-text_w)/2:y=(h-text_h)/2"
  local text="${file_count},${notification_filter}"

  # If we're in advanced mode
  if [[ $advanced = true ]]; then
    filters=(
      "[0:v]split=2[webcam][webcam_thumb]"
      "[1:v]split=2[latest][latest_thumb]"
      "[webcam_thumb]${thumb_fmt},${direction_filter}[webcam_thumb_filtered]"
      "[latest]${main_fmt}[overlay]"
      "[latest_thumb]${thumb_fmt}[latest_thumb_scaled]"
      "[webcam]${main_fmt},${webcam_filter}[webcam_filtered]"
      "[webcam_filtered][overlay]${onion_filter}[mux]"
      "[webcam_thumb_filtered][latest_thumb_scaled]vstack=inputs=2[left_stack]"
      "[mux]${text}[main]"
      "[left_stack][main]hstack=inputs=2[output]"
    )
  else
    filters=(
      "[0:v]${main_fmt},${webcam_filter}[webcam]"
      "[1:v]${main_fmt}[latest]"
      "[webcam][latest]${onion_filter}[mux]"
      "[mux]${text}[output]"
    )
  fi

  # Join filters into single string
  local filter_complex=$(
    IFS=\;
    echo "${filters[*]}"
  )

  # Start virtual camera with overlay
  ffmpeg -f v4l2 -input_format mjpeg -video_size 1920x1080 -framerate 30 -i "$device_w" \
    -loop 1 -i $SYMLINK \
    -filter_complex "$filter_complex" -map "[output]" \
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
  # Handle triggers
  if [[ -f "$TRIGGER_CAPTURE" ]]; then
    rm -f "$TRIGGER_CAPTURE"
    capture
  fi

  if [[ -f "$TRIGGER_DELETION" ]]; then
    rm -f "$TRIGGER_DELETION"
    delete
  fi

  if [[ -f "$TRIGGER_PLAYBACK" ]]; then
    rm -f "$TRIGGER_PLAYBACK"
    playback
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
