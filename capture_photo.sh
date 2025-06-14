#!/usr/bin/env bash

capture_photo() {
  local frames=$(read_tmp frames.count)
  local dir=$(read_tmp frames.dir)
  local prefix=$(read_tmp frames.prefix)
  local resolution=$(read_tmp preview.resolution)
  local device=$(read_tmp camera.device)

  if [[ -z $frames || -z $dir || -z $prefix || -z $resolution ]]; then
    echo "Missing variables. Can't capture."
    echo "$frames"
    echo "$dir"
    echo "$prefix"
    echo "$resolution"
    echo "$device"
    exit 1
  fi

  local new_frame=$((frames + 1))
  # Photo capture function
  local filename="$prefix_$(printf "%04d" $new_frame).bmp"
  local filepath="$(read_tmp frames.dir)/$filename"

  echo "📸 $filename"

  # Pause the process
  kill_stream
  echo "puased.... waiting"
  sleep 2
  # Capture frame
  ffmpeg -f v4l2 -pixel_format yuyv422 -video_size $resolution -i $device -frames:v 1 $filepath -y
  write_tmp $new_count frames.count
  # v4l2-ctl --device=$device --set-fmt-video=width=1920,height=1080,pixelformat=YUYV --stream-mmap --stream-count=1 --stream-to=$filepath
  echo "tried to save... resuming"
  sleep 2
  # Resume the process
  overlay $filename

  # # Parse CAPTURE_FORMAT
  # local width height pixelformat
  #
  # for ((i = 0; i < ${#CF[@]}; i++)); do
  #   case "${CF[i]}" in
  #   "-video_size" | "-s")
  #     local size="${CF[i + 1]}"
  #     width=${size%x*}
  #     height=${size#*x}
  #     ;;
  #   "-input_format")
  #     case "${CF[i + 1]}" in
  #     "mjpeg" | "MJPEG") pixelformat="MJPG" ;;
  #     "yuyv422" | "YUYV") pixelformat="YUYV" ;;
  #     *) pixelformat="MJPG" ;;
  #     esac
  #     ;;
  #   esac
  # done

  # Capture with v4l2-ctl
  # v4l2-ctl --device="$WEBCAM_DEV" \
  #   --set-fmt-video=width=$width,height=$height,pixelformat=$pixelformat \
  #   --stream-mmap --stream-count=1 --stream-to="$filepath" >/dev/null 2>&1

  # Check if file was actually created (more reliable than exit code)
  echo "file at $filepath"
  if [[ -f "$filepath" && -s "$filepath" ]]; then
    echo "✅ Saved: $filename"
  else
    echo "❌ Capture failed - no file created"
  fi
}
