#!/usr/bin/env bash

previewer_pid=$(cat "/tmp/just_stop_previewer.pid")
if [[ -n "$previewer_pid" ]]; then
  echo "Sending capture signal to previewer (PID: $previewer_pid)"
  kill -USR1 "$previewer_pid"
else
  echo "No previewer process found! Start host first."
  exit 1
fi
