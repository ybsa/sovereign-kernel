//! OTTO's dynamic code and environment generation runtime.
//! Defines Otto's Outpost: Zero-Pollution Docker synthesis and native fallback execution.

use std::path::Path;
use std::time::Instant;
use tokio::fs;
use tokio::process::Command;
use tracing::{debug, info, warn};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq)]
pub enum ExecutionEnv {
    Docker,
    Native,
}

#[derive(Debug, Clone)]
pub struct OttosOutpostRequest {
    pub language: String,
    pub execution_env: ExecutionEnv,
    pub dependencies: Vec<String>,
    pub code: String,
    pub input_files: Vec<(String, String)>, // filename, content
}

#[derive(Debug, Clone)]
pub struct OttosOutpostResult {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
    pub execution_time_ms: u128,
}

#[derive(Debug, thiserror::Error)]
pub enum OttosOutpostError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Docker build failed: {0}")]
    DockerBuildFailed(String),
    #[error("Container execution failed: {0}")]
    ContainerExecFailed(String),
    #[error("Native setup failed: {0}")]
    NativeSetupFailed(String),
    #[error("Unsupported language: {0}")]
    UnsupportedLanguage(String),
}

pub async fn execute_ottos_outpost(
    req: OttosOutpostRequest,
    workspace_root: &Path,
) -> Result<OttosOutpostResult, OttosOutpostError> {
    let run_id = Uuid::new_v4().to_string();
    let outpost_dir = workspace_root
        .join("tmp")
        .join(format!("ottos_outpost_{}", &run_id[..8]));

    fs::create_dir_all(&outpost_dir).await?;

    for (filename, content) in &req.input_files {
        // Basic path traversal guard
        let safe_name = Path::new(filename).file_name().unwrap_or_default();
        if !safe_name.is_empty() {
            fs::write(outpost_dir.join(safe_name), content).await?;
        }
    }

    let lang = req.language.to_lowercase();
    let script_name = match lang.as_str() {
        "python" | "py" => "main.py",
        "node" | "javascript" | "js" => "main.js",
        "bash" | "sh" => "main.sh",
        "powershell" | "ps1" => "main.ps1",
        _ => return Err(OttosOutpostError::UnsupportedLanguage(req.language)),
    };

    fs::write(outpost_dir.join(script_name), &req.code).await?;

    let start = Instant::now();

    let exec_result = match req.execution_env {
        ExecutionEnv::Docker => {
            execute_in_docker(&req, &outpost_dir, script_name, &run_id, &lang).await
        }
        ExecutionEnv::Native => execute_natively(&req, &outpost_dir, script_name, &lang).await,
    };

    let duration_ms = start.elapsed().as_millis();

    // Attempt cleanup, but don't fail the response if cleanup fails (e.g. Windows file lock)
    if let Err(e) = fs::remove_dir_all(&outpost_dir).await {
        warn!(
            "Failed to clean up Otto's Outpost directory {}: {}",
            outpost_dir.display(),
            e
        );
    }

    let mut final_res = exec_result?;
    final_res.execution_time_ms = duration_ms;

    Ok(final_res)
}

async fn execute_in_docker(
    req: &OttosOutpostRequest,
    outpost_dir: &Path,
    script_name: &str,
    run_id: &str,
    lang: &str,
) -> Result<OttosOutpostResult, OttosOutpostError> {
    info!(
        "Synthesizing Zero-Pollution Docker environment for language: {}",
        lang
    );
    let image_name = format!("sk-ottos-outpost-{}", &run_id[..8]);

    // 1. Synthesize Dockerfile
    let mut dockerfile = String::new();
    match lang {
        "python" | "py" => {
            dockerfile.push_str("FROM python:3.11-slim\nWORKDIR /app\n");
            if !req.dependencies.is_empty() {
                let deps = req.dependencies.join(" ");
                dockerfile.push_str(&format!("RUN pip install --no-cache-dir {}\n", deps));
            }
            dockerfile.push_str(&format!(
                "COPY . .\nCMD [\"python\", \"{}\"]\n",
                script_name
            ));
        }
        "node" | "javascript" | "js" => {
            dockerfile.push_str("FROM node:20-slim\nWORKDIR /app\n");
            if !req.dependencies.is_empty() {
                let deps = req.dependencies.join(" ");
                dockerfile.push_str(&format!("RUN npm install {}\n", deps));
            }
            dockerfile.push_str(&format!("COPY . .\nCMD [\"node\", \"{}\"]\n", script_name));
        }
        "bash" | "sh" => {
            dockerfile.push_str("FROM ubuntu:22.04\nWORKDIR /app\n");
            if !req.dependencies.is_empty() {
                let deps = req.dependencies.join(" ");
                dockerfile.push_str("RUN apt-get update && apt-get install -y ");
                dockerfile.push_str(&deps);
                dockerfile.push_str(" && rm -rf /var/lib/apt/lists/*\n");
            }
            dockerfile.push_str(&format!("COPY . .\nCMD [\"bash\", \"{}\"]\n", script_name));
        }
        _ => return Err(OttosOutpostError::UnsupportedLanguage(lang.to_string())),
    }

    fs::write(outpost_dir.join("Dockerfile"), dockerfile).await?;

    // 2. Build the image
    debug!("Building Docker image: {}", image_name);
    let build_output = Command::new("docker")
        .current_dir(outpost_dir)
        .arg("build")
        .arg("-t")
        .arg(&image_name)
        .arg(".")
        .output()
        .await?;

    if !build_output.status.success() {
        let err = String::from_utf8_lossy(&build_output.stderr).into_owned();
        return Err(OttosOutpostError::DockerBuildFailed(err));
    }

    // 3. Run the container securely
    let run_output = Command::new("docker")
        .arg("run")
        .arg("--rm") // destroy after run
        .arg("--network")
        .arg("none") // isolated unless we explicitly open it (could be configurable later)
        .arg("--memory")
        .arg("512m")
        .arg(&image_name)
        .output()
        .await?;

    // 4. Destroy the synthesized image
    let _ = Command::new("docker")
        .arg("rmi")
        .arg(image_name)
        .output()
        .await;

    Ok(OttosOutpostResult {
        stdout: String::from_utf8_lossy(&run_output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&run_output.stderr).into_owned(),
        exit_code: run_output.status.code().unwrap_or(-1),
        execution_time_ms: 0,
    })
}

async fn execute_natively(
    req: &OttosOutpostRequest,
    outpost_dir: &Path,
    script_name: &str,
    lang: &str,
) -> Result<OttosOutpostResult, OttosOutpostError> {
    info!("Executing native synthesis block for language: {}", lang);

    // 1. Install dependencies
    if !req.dependencies.is_empty() {
        let (cmd, args) = match lang {
            "python" | "py" => ("pip", vec!["install"]),
            "node" | "javascript" | "js" => ("npm", vec!["install"]),
            _ => {
                warn!(
                    "Native dependencies provided for language {} but no package manager defined",
                    lang
                );
                ("", vec![])
            }
        };

        if !cmd.is_empty() {
            let mut install_cmd = Command::new(cmd);
            install_cmd
                .current_dir(outpost_dir)
                .args(&args)
                .args(&req.dependencies);

            let output = install_cmd.output().await?;
            if !output.status.success() {
                let err = String::from_utf8_lossy(&output.stderr).into_owned();
                return Err(OttosOutpostError::NativeSetupFailed(err));
            }
        }
    }

    // 2. Execute script
    let (exe, args) = match lang {
        "python" | "py" => ("python", vec![script_name]),
        "node" | "javascript" | "js" => ("node", vec![script_name]),
        "bash" | "sh" => ("bash", vec![script_name]),
        "powershell" | "ps1" => (
            "powershell",
            vec!["-ExecutionPolicy", "Bypass", "-File", script_name],
        ),
        _ => return Err(OttosOutpostError::UnsupportedLanguage(lang.to_string())),
    };

    let output = Command::new(exe)
        .current_dir(outpost_dir)
        .args(&args)
        .output()
        .await?;

    Ok(OttosOutpostResult {
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        exit_code: output.status.code().unwrap_or(-1),
        execution_time_ms: 0,
    })
}
