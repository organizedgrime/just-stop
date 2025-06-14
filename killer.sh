#!/usr/bin/env bash

cleanup() {
  echo "hard killing everything unresumable"
  local ffplay_pid=$(read_tmp ffplay.pid)
  local ffmpeg_pid=$(read_tmp ffmpeg.pid)
  local inotify_pid=$(read_tmp inotify.pid)
  [[ -n "$ffplay_pid" ]] && kill -KILL "$ffplay_pid"
  [[ -n "$ffmpeg_pid" ]] && kill -KILL "$ffmpeg_pid"
  [[ -n "$inotify_pid" ]] && kill -KILL "$inotify_pid"

  # Clean up TMP files
  rm -rf "/tmp/just_stop"

  exit 0
}

kill_stream() {
  local ffplay_pid=$(read_tmp ffplay.pid)
  local ffmpeg_pid=$(read_tmp ffmpeg.pid)
  echo "pausing stream while capturing. ffplay $ffplay_pid & ffmpeg $ffmpeg_pid"
  [[ -n "$ffplay_pid" ]] && kill -KILL "$ffplay_pid"
  [[ -n "$ffmpeg_pid" ]] && kill -KILl "$ffmpeg_pid"
}

# start_stream() {
#   local ffplay_pid=$(read_tmp ffplay.pid)
#   local ffmpeg_pid=$(read_tmp ffmpeg.pid)
#   echo "resuming stream after capturing. ffplay $ffplay_pid & ffmpeg $ffmpeg_pid"
#   [[ -n "$ffplay_pid" ]] && kill -CONT "$ffplay_pid"
#   [[ -n "$ffmpeg_pid" ]] && kill -CONT "$ffmpeg_pid"
# }

export -f cleanup
export -f pause_stream
# export -f resume_stream
