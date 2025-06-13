capture_photo() {
  local new_count=$((PREVIOUS_COUNT + 1))
  # Photo capture function
  local filename="frame_$(printf "%04d" $new_count).jpg"
  local filepath="$WATCH_DIR/$filename"

  echo "📸 $filename"

  # Check if preview was running
  local was_running=false
  if pgrep -f "ffmpeg.*$WEBCAM_DEV" >/dev/null; then
    local was_running=true
    echo "Pausing preview..."

    # Kill all camera-related processes
    pkill -TERM -f "ffmpeg.*$WEBCAM_DEV" 2>/dev/null
    pkill -TERM -f "ffplay.*$WINDOW_TITLE" 2>/dev/null

    # Wait for processes to exit
    local wait_count=0
    while pgrep -f "ffmpeg.*$WEBCAM_DEV" >/dev/null && [[ $wait_count -lt 20 ]]; do
      sleep 0.1
      ((wait_count++))
    done

    # Force kill if still running
    pkill -KILL -f "ffmpeg.*$WEBCAM_DEV" 2>/dev/null
    pkill -KILL -f "ffplay.*$WINDOW_TITLE" 2>/dev/null
    sleep 0.3
  fi

  # Capture frame
  read -ra CF <<<"$CAPTURE_FORMAT"
  ffmpeg "${CF[@]}" -frames:v 1 -y "$filepath" -loglevel error

  # Restart preview if it was running
  if [[ "$was_running" == "true" && -n "$LATEST_IMAGE" ]]; then
    echo "Resuming preview..."
    overlay "$LATEST_IMAGE"
  fi

  # Check if file was actually created (more reliable than exit code)
  if [[ -f "$filepath" && -s "$filepath" ]]; then
    export PREVIOUS_COUNT=$new_count
    echo "✅ Saved: $filename"
  else
    echo "❌ Capture failed - no file created"
  fi
}
