#!/usr/bin/env bash

tmp="/tmp/just_stop"

# If the environment variable isn't set
if [[ -d "$tmp" ]]; then
  echo "Found existing TMP folder $tmp 📂"
else
  # Make a temporary directory for storing info while running
  mkdir $tmp
  echo "Created new temporary directory @ $tmp 📂"
fi

touch_tmp() {
  if [[ ! -f "$tmp/$1" ]]; then
    touch "$tmp/$1"
  fi
  echo -e "$tmp/$1"
}

# Read the contents of the specified file
read_tmp() {
  local file="$tmp/$1"
  [[ -f "$file" ]] && cat "$file"
}

# Silently save text to a file of the provided name
write_tmp() {
  if [[ -z $2 ]]; then
    echo "tried to write $1 to empty file name"
    exit 1
  else
    local file="$tmp/$2"
    echo $1 >$file
  fi
}

export -f touch_tmp
export -f read_tmp
export -f write_tmp
