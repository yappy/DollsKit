use crate::sys::config;
use crate::sys::taskserver::Control;
use super::SystemModule;
use anyhow::{Result, anyhow, ensure};
use chrono::NaiveTime;
use log::info;
use serde::{Serialize, Deserialize};
use tokio::process::Command;

#[derive(Clone, Serialize, Deserialize)]
struct HealthConfig {
    enabled: bool,
    debug_exec_once: bool,
}

pub struct Health {
    config: HealthConfig,
    wakeup_list: Vec<NaiveTime>,
}

impl Health {
    pub fn new(wakeup_list: Vec<NaiveTime>) -> Result<Self> {
        info!("[health] initialize");

        let jsobj = config::get_object(&["health"])
            .map_or(Err(anyhow!("Config not found: health")), Ok)?;
        let config: HealthConfig = serde_json::from_value(jsobj)?;

        Ok(Health {
            config,
            wakeup_list,
        })
    }

    async fn health_task(&mut self, ctrl: &Control) -> Result<()> {
        let mem_info = get_mem_info().await;
        info!("{:?}", mem_info?);
        let disk_info = get_disk_info().await;
        info!("{:?}", disk_info?);

        Ok(())
    }

    async fn health_task_entry(ctrl: Control) -> Result<()> {
        let mut health = ctrl.sysmods().health.write().await;
        health.health_task(&ctrl).await
    }
}

impl SystemModule for Health {
    fn on_start(&self, ctrl: &Control) {
        info!("[health] on_start");
        if self.config.enabled {
            if self.config.debug_exec_once {
                ctrl.spawn_oneshot_task(
                    "health_check",
                    Health::health_task_entry);
            }
            else {
                ctrl.spawn_periodic_task(
                    "health_check",
                    &self.wakeup_list,
                    Health::health_task_entry);
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct MemInfo {
    total_mib: f64,
    avail_mib: f64,
    percent: f64,
}

async fn get_mem_info() -> Result<MemInfo> {
    let mut cmd = Command::new("free");
	let output = cmd.output().await?;
    ensure!(output.status.success(), "free command failed");

    // sample
    //                total        used        free      shared  buff/cache   available
    // Mem:        13034888     4119272     5561960          68     3353656     8609008
    // Swap:        4194304           0     4194304
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut total = None;
    let mut avail = None;
    for (line_no, line) in stdout.lines().enumerate() {
        if line_no != 1 {
            continue;
        }
        for (col_no, elem) in line.split_ascii_whitespace().enumerate() {
            match col_no {
                1 => total = Some(elem),
                6 => avail = Some(elem),
                _ => (),
            }
        }
        break;
    }
    let total = total.ok_or(anyhow!("parse error"))?;
    let avail = avail.ok_or(anyhow!("parse error"))?;
    let total_mib = total.parse::<u64>()? as f64 / 1024.0;
    let avail_mib = avail.parse::<u64>()? as f64 / 1024.0;
    let percent = 100.0 * avail_mib / total_mib;

    Ok(MemInfo { total_mib, avail_mib, percent })
}

#[derive(Debug, Clone, Copy)]
struct DiskInfo {
    total_gib: f64,
    avail_gib: f64,
    percent: f64,
}

async fn get_disk_info() -> Result<DiskInfo> {
    let mut cmd = Command::new("df");
	let output = cmd.output().await?;
    ensure!(output.status.success(), "df command failed");

    // sample
    // ファイルシス   1K-ブロック     使用    使用可 使用% マウント位置
    // /dev/root        122621412 12964620 104641120   12% /
    // devtmpfs           1800568        0   1800568    0% /dev
    // tmpfs              1965432        0   1965432    0% /dev/shm
    // tmpfs              1965432    17116   1948316    1% /run
    // tmpfs                 5120        4      5116    1% /run/lock
    // tmpfs              1965432        0   1965432    0% /sys/fs/cgroup
    // /dev/mmcblk0p1      258095    49324    208772   20% /boot
    // /dev/sda1         59280316 57109344         0  100% /media/usbbkup
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut total = None;
    let mut avail = None;
    for line in stdout.lines().skip(1) {
        let mut total_tmp = None;
        let mut avail_tmp = None;
        let mut mp_tmp = None;
        for (col_no, elem) in line.split_ascii_whitespace().enumerate() {
            match col_no {
                1 => total_tmp = Some(elem),
                3 => avail_tmp = Some(elem),
                5 => mp_tmp = Some(elem),
                _ => (),
            }
        }
        if let Some(mp) = mp_tmp {
            if mp == "/" {
                total = total_tmp;
                avail = avail_tmp;
            }
        }
    }
    let total = total.ok_or(anyhow!("parse error"))?;
    let avail = avail.ok_or(anyhow!("parse error"))?;
    let total_gib = total.parse::<u64>()? as f64 / 1024.0 / 1024.0;
    let avail_gib = avail.parse::<u64>()? as f64 / 1024.0 / 1024.0;
    let percent = 100.0 * avail_gib / total_gib;

    Ok(DiskInfo { total_gib, avail_gib, percent })
}
