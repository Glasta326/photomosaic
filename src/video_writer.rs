use std::{
    fs,
    io::Write,
    process::{Child, ChildStdin, Command, Stdio},
};

use crate::{config_parse::Config, gpu::GpuContext};

pub struct VideoWriter {
    process: Child,
    stdin: ChildStdin,
}

impl VideoWriter {
    pub fn new(cfg: &Config, context: &GpuContext) -> std::io::Result<Self> {
        let file = format!(
            "{}",
            cfg.profile_log_fp
                .join(cfg.target_texture.file_prefix().unwrap())
                .with_extension("mp4")
                .to_string_lossy()
                .into_owned()
        );

        // Make sure to clear out the file if it already exists
        if fs::exists(&file).is_ok_and(|x| x == true) {
            fs::remove_file(&file)?;
        }

        let mut process = Command::new("ffmpeg")
            .args([
                "-f",
                "rawvideo",
                "-pixel_format",
                "rgba",
                "-video_size",
                &format!(
                    "{}x{}",
                    context.buffers.output_canvas_texture.width(),
                    context.buffers.output_canvas_texture.height()
                ),
                "-framerate",
                &60.to_string(),
                "-i",
                "-",
                // Make odd dimensions compatible with H.264.
                "-vf",
                "pad=ceil(iw/2)*2:ceil(ih/2)*2",
                "-c:v",
                "libx264",
                "-pix_fmt",
                "yuv420p",
                &file,
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()?;

        let stdin = process.stdin.take().unwrap();

        return Ok(VideoWriter { process, stdin });
    }

    pub fn write_frame(&mut self, frame: Vec<u8>) -> std::io::Result<()> {
        self.stdin.write_all(&frame)?;
        return Ok(());
    }

    // Note that this owns self, rather than taking in &self
    pub fn finish(mut self) -> std::io::Result<()> {
        drop(self.stdin);
        self.process.wait()?;
        return Ok(());
    }

    /// returns false if ffmpeg failed to run
    pub fn confirm_ffmpeg() -> bool {
        // Attempt to run ffmpeg -version command
        let mut cmd = Command::new("ffmpeg");
        cmd.arg("-version");
        let result = cmd.output();
        return result.is_ok();
    }
}

pub static mut X: i32 = 0;

pub fn set(x: i32) {
    unsafe {
        X = x;
    }
}

pub fn get() -> i32 {
    unsafe {
        return X;
    }
}
