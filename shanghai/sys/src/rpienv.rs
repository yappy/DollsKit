//! Raspberry Pi 固有の環境情報。

static RASPI_ENV: std::sync::OnceLock<RaspiEnv> = std::sync::OnceLock::new();

#[derive(Debug)]
pub enum RaspiEnv {
    NotRasRi,
    RasRi {
        model: String,
        cameras: Vec<CameraInfo>,
    },
}

impl std::fmt::Display for RaspiEnv {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RaspiEnv::NotRasRi => write!(f, "Not Raspberry Pi"),
            RaspiEnv::RasRi { model, cameras } => {
                writeln!(f, "Raspberry Model: {model}")?;

                writeln!(f, "Cameras:")?;
                for (i, cam) in cameras.iter().enumerate() {
                    write!(
                        f,
                        "{i}: model={}, resolution={}x{}",
                        cam.model, cam.width, cam.height
                    )?;
                    if i < cameras.len() - 1 {
                        writeln!(f)?;
                    }
                }
                Ok(())
            }
        }
    }
}

#[derive(Debug)]
pub struct CameraInfo {
    pub model: String,
    pub width: u32,
    pub height: u32,
}

fn get_env() -> RaspiEnv {
    // e.g. "Raspberry Pi 5 Model B Rev 1.1"
    let model = std::fs::read_to_string("/proc/device-tree/model")
        .map(|s| s.trim_end_matches('\0').to_string());

    match model {
        Ok(model) => {
            let cameras = get_camera_env().unwrap();
            RaspiEnv::RasRi { model, cameras }
        }
        Err(err) => {
            // NotFound は Raspberry Pi ではない正常環境
            // それ以外は panic
            if err.kind() == std::io::ErrorKind::NotFound {
                RaspiEnv::NotRasRi
            } else {
                panic!("{err}");
            }
        }
    }
}

fn get_camera_env() -> anyhow::Result<Vec<CameraInfo>> {
    let output = std::process::Command::new("rpicam-hello")
        .arg("--list-cameras")
        .output()?;
    anyhow::ensure!(output.status.success(), "rpicam-hello failed");
    let stdout = String::from_utf8_lossy(&output.stdout);

    parse_camera_list(&stdout)
}

/*
Sample:

Available cameras
-----------------
0 : imx500 [4056x3040 10-bit RGGB] (/base/axi/pcie@1000120000/rp1/i2c@88000/imx500@1a)
    Modes: 'SRGGB10_CSI2P' : 2028x1520 [30.02 fps - (0, 0)/4056x3040 crop]
                             4056x3040 [10.00 fps - (0, 0)/4056x3040 crop]
 */
fn parse_camera_list(stdout: &str) -> anyhow::Result<Vec<CameraInfo>> {
    static RE: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    let re = RE.get_or_init(|| regex::Regex::new(r"\s*\d+\s*:(.*)\[(\d+)x(\d+).*\]").unwrap());

    let mut res = Vec::new();
    for line in stdout.lines() {
        if let Some(caps) = re.captures(line) {
            let model = caps[1].trim().to_string();
            let width = caps[2].parse().unwrap();
            let height = caps[3].parse().unwrap();
            res.push(CameraInfo {
                model,
                width,
                height,
            });
        }
    }

    Ok(res)
}

pub fn raspi_env() -> &'static RaspiEnv {
    RASPI_ENV.get_or_init(get_env)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn raspi_env_not_panic() {
        let _env = raspi_env();
    }

    #[test]
    fn raspi_env_camera() {
        let sample = r"Available cameras
-----------------
0 : imx500 [4056x3040 10-bit RGGB] (/base/axi/pcie@1000120000/rp1/i2c@88000/imx500@1a)
    Modes: 'SRGGB10_CSI2P' : 2028x1520 [30.02 fps - (0, 0)/4056x3040 crop]
                             4056x3040 [10.00 fps - (0, 0)/4056x3040 crop]
";
        let cameras = parse_camera_list(sample).unwrap();
        assert_eq!(cameras.len(), 1);
        assert_eq!(cameras[0].model, "imx500");
        assert_eq!(cameras[0].width, 4056);
        assert_eq!(cameras[0].height, 3040);
    }
}
