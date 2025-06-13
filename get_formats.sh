#!/usr/bin/env bash
# Read device
d=$WEBCAM_DEV

# Get camera capabilities and parse with external awk script
formats=$(v4l2-ctl --device="$d" --list-formats-ext 2>/dev/null)

# Extract both formats
export PREVIEW_FORMAT=$(awk -v device="$d" -v prefer_lossy=1 -f parse_camera.awk <<<$formats)
export CAPTURE_FORMAT=$(awk -v device="$d" -v prefer_lossy=0 -f parse_camera.awk <<<$formats)

# If either of these fail
if [[ -z "$PREVIEW_FORMAT" || -z "$CAPTURE_FORMAT" ]]; then
  echo "Failed to get camera settings"
  exit 1
fi

# Extract preview resolution
export RESOLUTION=$(echo "$PREVIEW_FORMAT" | grep -o '[0-9]\+x[0-9]\+')
