cleanup() {
  # Cleanup function
  [[ "$CLEANUP_DONE" == "1" ]] && return
  export CLEANUP_DONE=1

  echo -e "\nStopping..."
  pkill -KILL -f "ffmpeg.*$WEBCAM_DEV"                 #2>/dev/null
  pkill -KILL -f "ffplay.*$WINDOW_TITLE"               # 2>/dev/null
  [[ -n "$INOTIFY_PID" ]] && kill -KILL "$INOTIFY_PID" #2>/dev/null

  # Clean up PID file
  rm -f "/tmp/just_stop_previewer.pid"

  exit 0
}
