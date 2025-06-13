#!/usr/bin/env bash

# Find the main previewer script process (not the child processes)
# Look for the one that has the directory argument
PREVIEWER_PID=$(pgrep -f "bash.*js\.sh.*/" | head -1)

if [[ -n "$PREVIEWER_PID" ]]; then
  echo "Sending capture signal to previewer (PID: $PREVIEWER_PID)"
  kill -USR1 "$PREVIEWER_PID"
else
  # Fallback: try any js.sh process and pick the first one
  PREVIEWER_PIDS=($(pgrep -f "js\.sh"))
  if [[ ${#PREVIEWER_PIDS[@]} -gt 0 ]]; then
    echo "Found multiple processes, using first one (PID: ${PREVIEWER_PIDS[0]})"
    kill -USR1 "${PREVIEWER_PIDS[0]}"
  else
    echo "No previewer process found!"
    exit 1
  fi
fi
