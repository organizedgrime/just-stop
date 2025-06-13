# #!/usr/bin/env bash
#
# # If the environment variable isn't set
# if [[ -z "$JS_TMP" ]]
#   # Make a temporary directory for storing info while running
#   export JS_TMP=$(mktemp /tmp/just_stop.XXXXXX)
#   echo "Created new temporary directory... 📂"
# fi
#
#
# # Silently save text to a file of the provided name
# write_tmp() {
#   echo $1 | tee "$JS_TMP/$2.txt" >/dev/null
#   read_tmp $2
# }
#
# # Read the contents of the specified file
# read_tmp() {
#   echo "$JS_TMP/$1.txt" || echo "DIED"
# }
#
# export -f write_tmp
# export -f read_tmp
