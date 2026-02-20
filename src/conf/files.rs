use anyhow::{Result, anyhow};
use crossbeam_channel::Receiver;
use ffmpeg_sidecar::{command::FfmpegCommand, pipe_name};
use std::{
    fs::{File, copy, create_dir_all, remove_dir_all, remove_file},
    io::Write,
    path::{Path, PathBuf},
    process::Command,
    time::{Duration, SystemTime},
};

use crate::{
    Message,
    conf::{effects::JustEffects, stream::JustStream},
};

#[derive(Clone)]
pub struct FileManager {
    pub prefix: String,
    pub photos: PathBuf,
    pub tmp: PathBuf,
}

impl FileManager {
    pub fn new(prefix: &str, photos: &str, tmp: &str) -> Result<Self> {
        let manager = Self {
            prefix: prefix.to_string(),
            photos: Path::new(photos).to_path_buf(),
            tmp: Path::new(tmp).to_path_buf(),
        };

        create_dir_all(&manager.tmp)?;
        create_dir_all(&manager.photos)?;
        File::create(&manager.notification())?;

        // Create a transparent image for the first snapshot
        FfmpegCommand::new()
            .format("lavfi")
            // .input("color=c=pink:s=1920x1080,format=rgba,colorchannelmixer=aa=0.2")
            .input("color=c=black:s=1920x1080,format=rgba,colorchannelmixer=aa=0.0")
            .frames(1)
            // .pix_fmt("bgra")
            .overwrite()
            .output(manager.transparent())
            .spawn()?
            .wait()?;

        // Copy the transparent image to the snapshot
        copy(manager.transparent(), &manager.snapshot())?;

        // The preview file needs to be a symlink to another image,
        // it can start as a symlink to the transparent one
        Self::symlink(&manager.transparent(), &manager.preview())?;

        // If there are photos
        if !manager.sorted_photos().is_empty() {
            // Symlink the most recent
            manager.symlink_latest()?;
        }

        // if Path::new(&manager.output_pipe()).exists() {
        //     remove_file(manager.output_pipe())?;
        // }
        //
        // Command::new("mkfifo").arg(manager.output_pipe()).output()?;

        // // The symlink is already good to go
        // if manager.latest_photo().is_none() {
        //     // Self::symlink(&manager.snapshot(), )
        //     manager.capture_photo()?;
        // } else {
        //     manager.symlink_latest()?;
        // }

        // if Path::new(&manager.latest()).exists() {
        //
        // } else {
        //     std::fs::copy(&manager.snapshot(), &manager.latest())?;
        // }

        Ok(manager)
    }

    fn tmp(&self, file_name: &str) -> String {
        self.tmp
            .join(format!("{file_name}"))
            .to_string_lossy()
            .to_owned()
            .to_string()
    }

    fn photos(&self, file_name: &str) -> String {
        self.photos
            .join(format!("{file_name}"))
            .to_string_lossy()
            .to_owned()
            .to_string()
    }

    pub fn notification(&self) -> String {
        self.tmp("notification.txt")
    }

    pub fn latest(&self) -> String {
        self.photos("latest.png")
    }

    pub fn snapshot(&self) -> String {
        self.tmp("snapshot.png")
    }

    // pub fn output_is_ready(&self) -> bool {
    //     Path::new(&self.output_socket_file()).exists()
    // }

    // pub fn output_socket(&self) -> String {
    //     format!("unix:{}", self.output_socket_file())
    // }
    //
    // pub fn output_socket_file(&self) -> String {
    //     self.tmp("output.socket")
    // }

    // pub fn output_pipe(&self) -> String {
    //     "/tmp/output.pipe".to_string()
    // }

    pub fn transparent(&self) -> String {
        self.photos("transparent.png")
    }

    pub fn preview(&self) -> String {
        self.photos("preview.png")
    }

    pub fn capture_trigger(&self) -> String {
        self.tmp("capture.trigger")
    }

    pub fn deletion_trigger(&self) -> String {
        self.tmp("deletion.trigger")
    }

    pub fn playback_trigger(&self) -> String {
        self.tmp("playback.trigger")
    }

    pub fn monitor_triggers(&self) -> Result<()> {
        // Clear the notification file
        File::create(&Path::new(&self.notification()))?;
        // Check for trigger files
        if Path::new(&self.capture_trigger()).exists() {
            remove_file(&self.capture_trigger())?;
            println!("\n📸 Capture triggered");
            if let Err(e) = self.capture_photo() {
                eprintln!("Capture failed: {}", e);
            }
            self.notify("capture")?;
        }

        if Path::new(&self.deletion_trigger()).exists() {
            remove_file(&self.deletion_trigger())?;
            println!("\n🗑️  Delete triggered");
            if let Err(e) = self.delete_latest_photo() {
                eprintln!("Delete failed: {}", e);
            }
            self.notify("delete")?;
        }

        if Path::new(&self.playback_trigger()).exists() {
            remove_file(&self.playback_trigger())?;
            println!("\n🎬 Playback triggered");

            for photo in self.sorted_photos() {
                if let Some(file_name) = photo.file_name()
                    && let Some(file_name) = file_name.to_str()
                {
                    // Link the file
                    Self::symlink(file_name, &self.preview())?;

                    // Wait for 1000/framerate millis
                    std::thread::sleep(Duration::from_millis(1000 / 12));
                }
            }

            // Once we're done, just make it transparent again
            Self::symlink(&self.transparent(), &self.preview())?;
        }
        Ok(())
    }

    /* pub fn create_playback(&self) -> Result<()> {
        // Remove the file if it already exists
        if Path::new(&self.playback()).exists() {
            remove_file(Path::new(&self.playback()))?;
        }

        FfmpegCommand::new()
            .args(["-framerate", "12"])
            .args(["-pattern_type", "glob"])
            .input(&self.photos(&format!("{}*.png", self.prefix)))
            .codec_video("libx264")
            .output(&self.playback())
            .print_command()
            .spawn()?
            .wait()?;
        Ok(())
    } */

    pub fn latest_photo(&self) -> Option<PathBuf> {
        std::fs::read_dir(&self.photos)
            .ok()?
            .filter_map(|entry| entry.ok())
            .filter_map(|entry| {
                match (
                    entry
                        .metadata()
                        .ok()
                        .and_then(|metadata| metadata.modified().ok()),
                    entry.file_type().ok(),
                ) {
                    (Some(modified), Some(typ)) => Some((entry.path(), modified, typ)),
                    _ => None,
                }
            })
            .filter_map(|(path, modified, typ)| {
                if let Some(ext) = path.extension().and_then(|ext| ext.to_str()) {
                    if ext == "png" && !typ.is_symlink() {
                        return Some((path, modified));
                    }
                }
                return None;
            })
            .max_by_key(|(_, modified)| *modified)
            .map(|(path, _)| path)
    }

    pub fn sorted_photos(&self) -> Vec<PathBuf> {
        let mut files: Vec<(PathBuf, SystemTime)> = vec![];

        if let Ok(results) = std::fs::read_dir(&self.photos) {
            for entry in results.filter_map(|entry| entry.ok()) {
                let path = entry.path();
                if let Ok(metadata) = path.metadata()
                    && let Ok(modified) = metadata.modified()
                    && let Ok(file_type) = entry.file_type()
                    && let Some(ext) = path.extension()
                    && ext == "png"
                    && !file_type.is_symlink()
                {
                    files.push((path, modified));
                }
            }
            files.sort_by_key(|(_, modified)| *modified);
        }

        return files.into_iter().map(|(path, _)| path).collect();
    }

    fn photo_count(&self) -> usize {
        std::fs::read_dir(&self.photos)
            .map(|entries| {
                entries
                    .filter_map(|entry| entry.ok())
                    .filter(|entry| {
                        entry
                            .path()
                            .extension()
                            .and_then(|ext| ext.to_str())
                            .map(|ext| ext == "png")
                            .unwrap_or(false)
                    })
                    .count()
            })
            .unwrap_or(0)
    }

    fn symlink(file: &str, link: &str) -> Result<()> {
        // TODO: consider compat with other non-local files
        let file_name = file.split("/").last().unwrap_or(file);
        Command::new("ln")
            .arg("-sf")
            .args([file_name, link])
            .output()?;
        println!("✓ Symlinked: {:?} to {}", file_name, link);
        Ok(())
    }

    fn symlink_latest(&self) -> Result<()> {
        if let Some(latest) = self.latest_photo()
            && let Some(file_name) = latest.file_name()
            && let Some(file_name) = file_name.to_str()
        {
            // Create symlink with just the filename (not full path)
            Self::symlink(file_name, &self.latest())?;
            Ok(())
        } else {
            Err(anyhow!("unable to symlink latest"))
        }
    }

    fn capture_photo(&self) -> Result<()> {
        let timestamp = chrono::Local::now().format("%Y_%m_%d_%H_%M_%S");
        let count = self.photo_count();
        let filename_only = format!("{}_{:03}_{}.png", self.prefix, count, timestamp);
        let filename = self.photos(&filename_only);

        println!("Capturing photo to {}...", filename);

        // Copy the snapshot we already have saved over to the new file name
        Command::new("cp")
            .arg(self.snapshot())
            .arg(&filename)
            .output()?;

        // if output.status.success() {
        println!("✓ Captured: {}", filename);

        // Create symlink with just the filename (not full path)
        self.symlink_latest()?;

        Ok(())
    }

    fn delete_latest_photo(&self) -> Result<()> {
        if let Some(latest) = self.latest_photo() {
            println!("Deleting {}...", latest.display());
            std::fs::remove_file(&latest)?;
            println!("✓ Deleted");
            self.symlink_latest()?;
            Ok(())
        } else {
            anyhow::bail!("No photos to delete")
        }
    }

    fn notify(&self, message: &str) -> Result<()> {
        let path = Path::new(&self.notification()).to_path_buf();
        let mut file = File::create(&path)?;
        file.write_all(message.as_bytes())?;
        std::thread::sleep(Duration::from_millis(333));
        file.flush()?;
        Ok(())
    }

    pub fn cleanup(&self) -> Result<()> {
        remove_dir_all(&self.tmp)?;
        Ok(())
    }

    pub fn build_command(
        &self,
        input: &str,
        stream: &JustStream,
        effects: &JustEffects,
    ) -> FfmpegCommand {
        let mut command = FfmpegCommand::new();
        // ffmpeg -f v4l2 -i /dev/video3 -f rawvideo -pix_fmt yuv420p -y /tmp/output.pipe
        // ffmpeg -f v4l2 -i /dev/video3   -f mpegts   -codec:v libx264 -preset ultrafast -tune zerolatency   -g 1   -bf 0   -fflags nobuffer   -flush_packets 1   udp://127.0.0.1:8090?pkt_size=1316
        command
            .format("v4l2")
            .input(&input)
            // .args(["-loop", "1"])
            // .input(&self.latest())
            // .args(["-loop", "1"])
            // .input(&self.preview())
            // .filter_complex(effects.filter_complex(&self))
            // .map("[output]")
            // .format("rawvideo")
            .format("mpegts")
            .codec_video("libx264")
            .preset("ultrafast")
            .args(["-tune", "zerolatency"])
            .args(["-g", "1"])
            .args(["-bf", "0"])
            .args(["-fflags", "nobuffer"])
            .args(["-flush_packets", "1"])
            // .pix_fmt("yuv420p")
            // .overwrite()
            .output(&format!("{}?pkt_size=1316", stream.to_string()))
            // .map("[snapshot]")
            // .rate(1.0)
            // .args(["-update", "1"])
            // .overwrite()
            // .output(&self.snapshot())
            .print_command();
        /* command
        .format("v4l2")
        // .realtime()
        // .pix_fmt(&pixfmt)
        // .args(["-input_format", "nv12"])
        // .args(["-video_size", "1920x1080"])
        // .args(["-framerate", &framerate])
        .input(&input)
        // .realtime()
        // .arg("-y")
        // .args(["-loop", "1"])
        // .args(["-f", "image2"])
        .input(&self.latest())
        // .realtime()
        // .args(["-video_size", "1920x1080"])
        // .args(["-loop", "1"])
        // .rate(24.0)
        // .args(["-loop", "1"])
        // // .args(["-framerate", "24"])
        // .args(["-f", "image2"])
        .input(&self.preview())
        .filter_complex(effects.filter_complex(&self))
        // .args(["-fflags", "+genpts+nobuffer"])
        // .args(["-flags", "lowdelay"])
        // .args(["-probesize", "32"])
        // .args(["-analyzeduration", "0"])
        // .args(["-use_wallclock_as_timestamps", "1"])
        .map("[output]")
        // .codec_video("libx264")
        // .args(["-tune", "zerolatency"])
        // .args(["-bf", "0"])
        // .args(["-g", "15"])
        // .preset("ultrafast")
        // .args(["-x264-params", "repeat-headers=1:bframes=0"])
        // .rate(24.0)
        // .format("mpegts")
        // .args(["-listen", "1"])
        // .output(&format!("unix:{}", self.output_socket_file()))
        .format("rawvideo")
        .pix_fmt("yuv420p")
        .args(["-listen", "1"])
        .output(&self.output_pipe())
        // .output(&format!("unix:{}", self.output_socket_file()))
        .map("[snapshot]")
        .rate(1.0)
        .args(["-update", "1"])
        .arg("-y")
        .output(&self.snapshot())
        .print_command(); */
        command
    }
}
