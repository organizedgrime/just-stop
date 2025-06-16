#!/usr/bin/awk -f

# Pass in virtual=1 for loopback cams, virtual=0 for real cams
BEGIN {
  want = (virtual == "1")
}

# On a device header line, set flag if it’s loopback
/^[^\t]/ {
  loopback = (/v4l2loopback/)
}

# On a device path line, print if it matches want
/^\t\/dev\/video[0-9]+/ {
  if (loopback == want)
    print $1
}
