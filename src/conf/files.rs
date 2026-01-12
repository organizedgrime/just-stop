use anyhow::{anyhow, Result};
use crossbeam_channel::Receiver;
use ffmpeg_sidecar::command::FfmpegCommand;
use std::{
    fs::{create_dir_all, remove_dir_all, remove_file, File},
    io::Write,
    path::{Path, PathBuf},
    process::Command,
    time::Duration,
};

use crate::Message;

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

        // The symlink is already good to go
        if manager.latest_photo().is_none() {
            // Create a transparent image for the first snapshot
            FfmpegCommand::new()
                .format("lavfi")
                .input("color=black@0.0:s=1920x1080")
                .frames(1)
                .pix_fmt("bgra")
                .arg("-y")
                .output(manager.snapshot())
                .spawn()?
                .wait()?;
            manager.capture_photo()?;
        } else {
            manager.symlink_latest()?;
        }

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
        self.photos("latest.bmp")
    }

    pub fn snapshot(&self) -> String {
        self.tmp("snapshot.bmp")
    }

    pub fn capture_trigger(&self) -> String {
        self.tmp("capture.trigger")
    }

    pub fn deletion_trigger(&self) -> String {
        self.tmp("deletion.trigger")
    }

    pub fn playback_trigger(&self) -> String {
        self.tmp("deletion.trigger")
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

            // if let Err(e) = self.create_playback(&output_path) {
            //     eprintln!("Playback failed: {}", e);
            // }
            //
            // Break to restart stream
        }

        Ok(())
    }

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
                    if ext == "bmp" && !typ.is_symlink() {
                        return Some((path, modified));
                    }
                }
                return None;
            })
            .max_by_key(|(_, modified)| *modified)
            .map(|(path, _)| path)
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
                            .map(|ext| ext == "bmp")
                            .unwrap_or(false)
                    })
                    .count()
            })
            .unwrap_or(0)
    }

    // fn symlink(&self) -> Result<()> {
    //         // Create symlink with just the filename (not full path)
    //         Command::new("ln")
    //             .arg("-sf")
    //             .args([
    //                 latest
    //                     .file_name()
    //                     .ok_or(anyhow!("no file name"))?
    //                     .to_str()
    //                     .ok_or(anyhow!("no file name"))?,
    //                 &self.latest(),
    //             ])
    //             .output()?;
    //
    //         println!("✓ Symlinked: {:?} to {}", latest, self.latest());
    //     }
    //     Ok(())
    // }

    fn symlink_latest(&self) -> Result<()> {
        if let Some(latest) = self.latest_photo() {
            // Create symlink with just the filename (not full path)
            Command::new("ln")
                .arg("-sf")
                .args([
                    latest
                        .file_name()
                        .ok_or(anyhow!("no file name"))?
                        .to_str()
                        .ok_or(anyhow!("no file name"))?,
                    &self.latest(),
                ])
                .output()?;

            println!("✓ Symlinked: {:?} to {}", latest, self.latest());
        }
        Ok(())
    }

    fn capture_photo(&self) -> Result<()> {
        let timestamp = chrono::Local::now().format("%Y_%m_%d_%H_%M_%S");
        let count = self.photo_count();
        let filename_only = format!("{}_{:03}_{}.bmp", self.prefix, count, timestamp);
        let filename = self.photos(&filename_only);

        println!("Capturing photo to {}...", filename);

        // Copy the snapshot we already have saved over to the new file name
        Command::new("cp")
            .arg(self.snapshot())
            .arg(&filename)
            .output()?;

        // if output.status.success() {
        println!("✓ Captured: {}", filename);

        // Remove old symlink if it exists
        let _ = std::fs::remove_file(self.latest()).ok();

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
}
