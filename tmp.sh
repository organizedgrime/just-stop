#!/usr/bin/env bash

folder_name="/tmp/just_stop_folder"

# If the environment variable isn't set
if [[ -z "$JS_TMP" || ! -d "$JS_TMP" ]]; then
  if [[ -f $folder_name ]]; then
    export JS_TMP=$(cat "$folder_name")
    echo "Found existing TMP folder $JS_TMP 📂"
  else
    # Make a temporary directory for storing info while running
    export JS_TMP=$(mktemp -d /tmp/just_stop.XXXXXX)
    echo $JS_TMP >$folder_name
    echo "Created new temporary directory @ $JS_TMP 📂"
  fi
fi

# Read the contents of the specified file
read_tmp() {
  local file="$JS_TMP/$1.txt"
  [[ -f "$file" ]] && cat "$file"
}

# Silently save text to a file of the provided name
write_tmp() {
  echo $1 >"$JS_TMP/$2.txt"
  read_tmp $2
}

export -f read_tmp
export -f write_tmp
