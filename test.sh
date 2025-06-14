RESOLUTION="1920x1080"
IMG=~/Pictures/tragedy/tragedy-8.jpg

ffmpeg -f v4l2 -input_format mjpeg -video_size $RESOLUTION -framerate 30 -i /dev/video0 \
  -loop 1 -i "$IMG" \
  -filter_complex "[1:v]scale=$RESOLUTION,format=yuva420p,colorchannelmixer=aa=0.5[bg];[0:v][bg]overlay,format=yuv420p" \
  -f nut - | ffplay -f nut -

# ffplay \
#   -fflags nobuffer -flags low_delay -framedrop -infbuf \
#   -f v4l2 -input_format mjpeg -video_size $RESOLUTION -framerate 30 -i /dev/video0 \
#   -vf "movie=$img:loop=0,scale=$RESOLUTION,format=rgba,colorchannelmixer=aa=0.5[bg];[in][bg]overlay" \
#   -window_title Preview
