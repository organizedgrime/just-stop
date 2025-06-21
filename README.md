# Just Stop

![demo](https://github.com/user-attachments/assets/22856278-0534-4d5c-a87a-ca52aefce01d)

`just_stop` is stop motion software for when you've given up.

> I just want to run stop motion software on linux, that should be fine right?

Think again! everything sucks! Maybe you like `DragonFrame`, more power too ya.
I dont need all that.

> I can just use a camera viewer like `cheese` or `guvcview`

Maybe you can! I got away with this for a long time. But, critically, these options do not support onion skinning.
Additionally, there's no distinction between capture format and preview format.
If you want a 30fps preview window, that means your individual frames will be stored as terrible lossy MJPEGs.
`just_stop` gets around this by terminating the preview process and then asking your webcam for a RAW photo before resuming the preview.

`just_stop` is just stop motion.
If you want to control your camera settings, use `cameractrls`.

> What do I need

FFMPEG and v4l2loopback. That's about it.

> How do i use it?

Run `just_stop.sh`. You now have a virtual camera.

Run `touch /tmp/just_stop/capture.trigger` to capture a frame.
Run `touch /tmp/just_stop/delete.trigger` to delete the most recent frame.
Run `touch /tmp/just_stop/playback.trigger` to render all the previously captured frames and play them back before returning to the preview.
