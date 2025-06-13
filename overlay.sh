overlay() {
  # Start/restart ffmpeg with overlay
  local img="$1"

  # Kill any existing camera processes
  pkill -f "ffmpeg.*$WEBCAM_DEV"   #2>/dev/null
  pkill -f "ffplay.*$WINDOW_TITLE" #2>/dev/null
  sleep 0.2

  read -ra PF <<<"$PREVIEW_FORMAT"
  if [[ -n "$img" ]]; then
    # Build common ffmpeg command (using smooth preview settings)
    local ffmpeg_cmd=(
      ffmpeg "${PF[@]}"
      -loop 1 -i "$img"
      -filter_complex "[1:v]scale=$RESOLUTION,format=yuva420p,colorchannelmixer=aa=0.5[overlay];[0:v][overlay]overlay"
    )
  else
    # NO OVERLAY: Direct camera passthrough
    echo "Starting camera preview (no overlay)"

    local ffmpeg_cmd=(
      ffmpeg "${PF[@]}"
    )
  fi

  if [[ $VIRTUAL_MODE -eq 1 ]]; then
    # Virtual camera output
    "${ffmpeg_cmd[@]}" -pix_fmt yuyv422 -f v4l2 "$VIRTUAL_DEV" 2>/dev/null &
  else
    # Preview window output
    "${ffmpeg_cmd[@]}" -f nut -c:v rawvideo - 2>/dev/null |
      ffplay -loglevel error -vf setpts=0 -window_title "$WINDOW_TITLE" - 2>/dev/null &
  fi
  export FFMPEG_PID=$!
}
