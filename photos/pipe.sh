if [[ ! -z /tmp/latest-pipe ]]; then
  rm /tmp/latest-pipe
fi

mkfifo /tmp/latest-pipe

while true; do
  convert "$(readlink -f ./latest.png)" -resize 1920x1080! rgb:-
  sleep 1
done >/tmp/latest-pipe
