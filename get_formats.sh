#!/usr/bin/env bash

get_formats() {
  # Read device
  local device=$(read_tmp camera.device)
  if [[ -z $device ]]; then
    echo "Missing variables. Can't determine formats."
    exit 1
  fi
  # Get camera capabilities and parse with external awk script
  local formats=$(v4l2-ctl --device="$device" --list-formats-ext 2>/dev/null)

  # Extract both formats
  write_tmp "$(awk -v device="$device" -v prefer_lossy=1 -f parse_camera.awk <<<$formats)" preview.format
  write_tmp "$(awk -v device="$device" -v prefer_lossy=0 -f parse_camera.awk <<<$formats)" capture.format
  local preview_format=$(read_tmp preview.format)
  local capture_format=$(read_tmp capture.format)
  echo "Preview format: $preview_format"
  echo "Capture format: $capture_format"

  # If either of these fail
  if [[ -z $preview_format || -z $capture_format ]]; then
    echo "Failed to get camera settings"
    exit 1
  else
    echo "Preview format: $preview_format"
    echo "Capture format: $capture_format"
  fi

  # Extract preview resolution
  write_tmp $(echo "$preview_format" | grep -o '[0-9]\+x[0-9]\+') preview.resolution
  local resolution=$(read_tmp preview.resolution)
}
