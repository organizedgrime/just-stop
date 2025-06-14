#!/usr/bin/env bash

overlay() {
  # Start/restart ffmpeg with overlay
  local img="$1"
  local virtual=$(read_tmp camera.virtual)
  local device=$(read_tmp camera.device)
  local resolution=$(read_tmp preview.resolution)
  local preview_format=$(read_tmp preview.format)

  if [[ -z $device || -z resolution || -z preview_format ]]; then
    echo "Missing variables. Can't overlay."
    exit 1
  fi
  #
  # # Kill any existing camera processes
  local ffplay_pid=$(read_tmp ffplay.pid)
  local ffmpeg_pid=$(read_tmp ffmpeg.pid)
  if [[ -n ffplay_pid ]]; then
    echo "killing ffplay at $ffplay_pid"
    kill -KILL $ffplay_pid 2>/dev/null
  fi
  if [[ -n ffmpeg_pid ]]; then
    echo "killing ffmpeg_pid at $ffmpeg_pid"
    kill -KILL $ffmpeg_pid 2>/dev/null
  fi

  local preview_args="$preview_format"
  # If no images was provided for overlay
  if [[ -n "$img" ]]; then
    # Build common ffmpeg command (using smooth preview settings)
    preview_args="$preview_format -loop 1 -i $img -filter_complex [1:v]scale=$resolution,format=yuva420p,colorchannelmixer=aa=0.5[overlay];[0:v][overlay]overlay"
  else
    # NO OVERLAY: Direct camera passthrough
    echo "Starting camera preview (no overlay)"
  fi
  echo "args: $preview_args"

  echo "making container..."
  local container=$(touch_tmp output.mov)
  echo "made container..."
  read -ra PA <<<$preview_args
  if [[ $virtual -eq 1 ]]; then
    # Virtual camera output
    ffmpeg "${PA[@]}" -f v4l2 -pix_fmt yuyv422 "$device" -y "$container" &
    write_tmp $(pgrep -f "ffmpeg.*$device") ffmpeg.pid
  else
    if [[ -z $img ]]; then
      ffplay -f v4l2 -input_format mjpeg -video_size $resolution -framerate 30 -i /dev/video0 -fflags nobuffer >/dev/null &
    else
      # Preview window output
      ffplay -f v4l2 -input_format mjpeg -video_size $resolution -framerate 30 -i /dev/video0 \
        -vf "movie=$img,loop=1,scale=$resolution,format=yuva420p,colorchannelmixer=aa=0.5[bg];[in][bg]overlay" \
        -fflags nobuffer >/dev/null &
    fi
    write_tmp $! ffplay.pid
  fi
  echo "ffmpeg pid $(read_tmp ffmpeg.pid)"
  echo "ffplay pid $(read_tmp ffplay.pid)"
}
