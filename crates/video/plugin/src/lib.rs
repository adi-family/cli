use lib_plugin_prelude::*;

#[derive(CliArgs)]
pub struct RenderArgs {
    #[arg(position = 0)]
    pub composition: String,

    #[arg(long, default = "mp4".to_string())]
    pub format: String,

    #[arg(long, default = 30)]
    pub fps: i64,

    #[arg(long, default = 1920)]
    pub width: i64,

    #[arg(long, default = 1080)]
    pub height: i64,

    #[arg(long, default = 150)]
    pub duration: i64,

    #[arg(long, default = 23)]
    pub crf: i64,
}

#[derive(CliArgs)]
pub struct StatusArgs {
    #[arg(position = 0)]
    pub job_id: String,
}

#[derive(CliArgs)]
pub struct DownloadArgs {
    #[arg(position = 0)]
    pub job_id: String,

    #[arg(long = "output", short = 'o')]
    pub output: Option<String>,
}

pub struct VideoPlugin;

impl VideoPlugin {
    pub fn new() -> Self {
        Self
    }
}

impl Default for VideoPlugin {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Plugin for VideoPlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata::new("adi.video", "Video", env!("CARGO_PKG_VERSION"))
            .with_type(PluginType::Core)
            .with_author("ADI Team")
            .with_description("Programmatic video rendering with FFmpeg")
    }

    async fn init(&mut self, _ctx: &PluginContext) -> Result<()> {
        Ok(())
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    fn provides(&self) -> Vec<&'static str> {
        vec![SERVICE_CLI_COMMANDS]
    }
}

#[async_trait]
impl CliCommands for VideoPlugin {
    async fn list_commands(&self) -> Vec<CliCommand> {
        vec![
            Self::__sdk_cmd_meta_render(),
            Self::__sdk_cmd_meta_status(),
            Self::__sdk_cmd_meta_list(),
            Self::__sdk_cmd_meta_download(),
        ]
    }

    async fn run_command(&self, ctx: &CliContext) -> Result<CliResult> {
        match ctx.subcommand.as_deref() {
            Some("render") => self.__sdk_cmd_handler_render(ctx).await,
            Some("status") => self.__sdk_cmd_handler_status(ctx).await,
            Some("list") => self.__sdk_cmd_handler_list(ctx).await,
            Some("download") => self.__sdk_cmd_handler_download(ctx).await,
            Some(cmd) => Ok(CliResult::error(format!("Unknown command: {cmd}"))),
            None => Ok(CliResult::success(self.help())),
        }
    }
}

impl VideoPlugin {
    fn help(&self) -> String {
        "ADI Video — Programmatic video rendering\n\n\
         Commands:\n  \
           render   Start a video render job\n  \
           status   Check render job status\n  \
           list     List all render jobs\n  \
           download Download a completed render\n\n\
         Usage: adi video <command> [options]"
            .to_string()
    }

    fn base_url(&self) -> String {
        std::env::var("ADI_VIDEO_URL")
            .unwrap_or_else(|_| "http://localhost:3100".to_string())
    }

    #[command(name = "render", description = "Start a video render job")]
    async fn render(&self, args: RenderArgs) -> CmdResult {
        let url = format!("{}/v1/render", self.base_url());

        let body = serde_json::json!({
            "width": args.width,
            "height": args.height,
            "fps": args.fps,
            "total_frames": args.duration,
            "format": args.format,
            "crf": args.crf,
        });

        let client = reqwest::Client::new();
        let resp = client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("Request failed: {e}"))?;

        if !resp.status().is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(format!("Render failed: {text}"));
        }

        let data: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("Invalid response: {e}"))?;

        let job_id = data["job_id"]
            .as_str()
            .unwrap_or("unknown");

        Ok(format!(
            "Render job created: {job_id}\n\
             Composition: {}\n\
             Format: {} | {}x{} @ {}fps | {} frames\n\n\
             Check status: adi video status {job_id}",
            args.composition,
            args.format,
            args.width,
            args.height,
            args.fps,
            args.duration,
        ))
    }

    #[command(name = "status", description = "Check render job status")]
    async fn status(&self, args: StatusArgs) -> CmdResult {
        let url = format!("{}/v1/render/{}", self.base_url(), args.job_id);

        let client = reqwest::Client::new();
        let resp = client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("Request failed: {e}"))?;

        if !resp.status().is_success() {
            return Err(format!("Job not found: {}", args.job_id));
        }

        let data: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("Invalid response: {e}"))?;

        let phase = data["phase"].as_str().unwrap_or("unknown");
        let progress = data["progress"].as_f64().unwrap_or(0.0);
        let frames = data["framesReceived"].as_u64().unwrap_or(0);
        let total = data["totalFrames"].as_u64().unwrap_or(0);

        Ok(format!(
            "Job: {}\n\
             Phase: {phase}\n\
             Progress: {:.0}%\n\
             Frames: {frames}/{total}",
            args.job_id,
            progress * 100.0,
        ))
    }

    #[command(name = "list", description = "List all render jobs")]
    async fn list(&self) -> CmdResult {
        let url = format!("{}/v1/jobs", self.base_url());

        let client = reqwest::Client::new();
        let resp = client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("Request failed: {e}"))?;

        let data: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| format!("Invalid response: {e}"))?;

        let jobs = data["jobs"].as_array();
        match jobs {
            Some(jobs) if jobs.is_empty() => Ok("No render jobs".to_string()),
            Some(jobs) => {
                let mut output = String::from("Render Jobs\n\n");
                for job in jobs {
                    let id = job["id"].as_str().unwrap_or("?");
                    let phase = job["phase"].as_str().unwrap_or("?");
                    let short_id = &id[..8.min(id.len())];
                    output.push_str(&format!("  {short_id}  {phase}\n"));
                }
                Ok(output.trim_end().to_string())
            }
            None => Err("Invalid response format".to_string()),
        }
    }

    #[command(name = "download", description = "Download a completed render")]
    async fn download(&self, args: DownloadArgs) -> CmdResult {
        let url = format!("{}/v1/render/{}/download", self.base_url(), args.job_id);

        let client = reqwest::Client::new();
        let resp = client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("Request failed: {e}"))?;

        if !resp.status().is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(format!("Download failed: {text}"));
        }

        let filename = args.output.unwrap_or_else(|| {
            format!("render-{}.mp4", &args.job_id[..8.min(args.job_id.len())])
        });

        let bytes = resp
            .bytes()
            .await
            .map_err(|e| format!("Download failed: {e}"))?;

        std::fs::write(&filename, &bytes)
            .map_err(|e| format!("Failed to write file: {e}"))?;

        Ok(format!("Downloaded to {filename} ({} bytes)", bytes.len()))
    }
}

#[no_mangle]
pub fn plugin_create() -> Box<dyn Plugin> {
    Box::new(VideoPlugin::new())
}

#[no_mangle]
pub fn plugin_create_cli() -> Box<dyn CliCommands> {
    Box::new(VideoPlugin::new())
}
