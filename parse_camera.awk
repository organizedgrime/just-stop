# Parse v4l2 camera capabilities and find optimal settings
# Usage: awk -f parse_camera.awk -v device="/dev/video0" -v prefer_lossy=0

# Track current format as we parse
/Motion-JPEG/ {
    current_format = "mjpeg"
    current_name = "MJPG"
}

/YUYV/ {
    current_format = "yuyv422"
    current_name = "YUYV"
}

# Extract resolution
/Size: Discrete/ {
    match($0, /([0-9]+)x([0-9]+)/, resolution)
    width = resolution[1]
    height = resolution[2]
    pixels = width * height
}

# Extract framerate and calculate best option
/fps\)/ {
    match($0, /\(([0-9.]+) fps\)/, framerate)
    fps = framerate[1]

    # Scoring: prioritize resolution, then format preference, then fps
    format_bonus = 0
    if (prefer_lossy && current_name == "MJPG") format_bonus = 100
    if (!prefer_lossy && current_name == "YUYV") format_bonus = 100

    score = pixels * 1000 + format_bonus + fps

    if (score > best_score) {
        best_score = score
        best_format = current_format
        best_resolution = width "x" height
        best_fps = int(fps)
    }
}

END {
    print "-f v4l2 -input_format " best_format " -video_size " best_resolution " -framerate " best_fps " -i " device
}
