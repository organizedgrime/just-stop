#!/usr/bin/env bash

source tmp.sh

host_pid=$(read_tmp host.pid)
if [[ -n "$host_pid" ]]; then
  echo "Sending capture signal to previewer (PID: $host_pid)"
  kill -USR1 "$host_pid"
else
  echo "No previewer process found! Start host first."
  exit 1
fi
