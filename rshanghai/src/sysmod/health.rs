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
        let cpu_info = get_cpu_info().await;
        info!("{:?}", cpu_info);
        let mem_info = get_mem_info().await;
        info!("{:?}", mem_info?);
        let disk_info = get_disk_info().await;
        info!("{:?}", disk_info?);
        let cpu_temp = get_cpu_temp().await;
        info!("{:?}", cpu_temp?);

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

#[derive(Debug, Clone)]
struct CpuInfo {
    usage_percent_total: f64,
    usage_percent_list: Vec<f64>,
}

async fn get_cpu_info() -> Result<CpuInfo> {
    let buf = tokio::fs::read("/proc/stat").await?;
    let text = String::from_utf8_lossy(&buf);

    // See `man proc`
    // user   (1) Time spent in user mode.
    // nice   (2) Time spent in user mode with low priority (nice).
    // system (3) Time spent in system mode.
    // idle   (4) Time spent in the idle task.  This value should be USER_HZ times the second entry in the /proc/uptime pseudo-file.
    // iowait (since Linux 2.5.41)
    //        (5) Time waiting for I/O to complete.  This value is not reliable, for the following reasons:
    // irq (since Linux 2.6.0-test4)
    //        (6) Time servicing interrupts.
    // softirq (since Linux 2.6.0-test4)
    //        (7) Time servicing softirqs.
    // steal (since Linux 2.6.11)
    //        (8)  Stolen  time,  which  is the time spent in other operating systems when running in a virtualized environment
    // guest (since Linux 2.6.24)
    //        (9) Time spent running a virtual CPU for guest operating systems under the control of the Linux kernel.
    // guest_nice (since Linux 2.6.33)
    //        (10)  Time spent running a niced guest (virtual CPU for guest operating systems under the
    //        control of the Linux kernel).
    let mut usage_percent_total = None;
    let mut usage_percent_list = vec![];
    for line in text.lines() {
        let mut name = None;
        let mut user = None;
        let mut nice = None;
        let mut system = None;
        let mut idle = None;
        for (col_no, elem) in line.split_ascii_whitespace().enumerate() {
            match col_no {
                0 => name = Some(elem),
                1 => user = Some(elem),
                2 => nice = Some(elem),
                3 => system = Some(elem),
                4 => idle = Some(elem),
                _ => (),
            }
        }
        // cpu or cpu%d の行を取り出す
        if name.is_none() || !name.unwrap().starts_with("cpu") {
            continue;
        }

        let user: u64 = user.ok_or(anyhow!("parse error"))?.parse()?;
        let nice: u64 = nice.ok_or(anyhow!("parse error"))?.parse()?;
        let system: u64 = system.ok_or(anyhow!("parse error"))?.parse()?;
        let idle: u64 = idle.ok_or(anyhow!("parse error"))?.parse()?;
        let total = user + nice + system + idle;
        let value = (total - idle) as f64 / total as f64;
        if name == Some("cpu") {
            usage_percent_total = Some(value);
        }
        else {
            usage_percent_list.push(value);
        }
    }

    ensure!(usage_percent_total.is_some());
    ensure!(!usage_percent_list.is_empty());
    Ok(CpuInfo {
        usage_percent_total: usage_percent_total.unwrap(),
        usage_percent_list,
    })
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

#[derive(Debug, Clone, Copy)]
struct CpuTemp {
    temp: f64,
}

/// CPU 温度 (正確には違うかもしれない。ボード上の何らかの温度センサの値。) を取得する。
///
/// "/sys/class/thermal/thermal_zone0/temp" による。
/// デバイスファイルが存在しないと [std::io::Error] (NotFound) になる。
/// Linux 汎用のようだが少なくとも WSL2 では存在しない。
/// RasPi only で `vcgencmd measure_temp` という手もあるが、
/// 人が読みやすい代わりにパースが難しくなるのでデバイスファイルの方を使う。
async fn get_cpu_temp() -> Result<CpuTemp> {
    let buf = tokio::fs::read("/sys/class/thermal/thermal_zone0/temp").await?;
    let text = String::from_utf8_lossy(&buf);

    // 'C を 1000 倍した整数が得られるので変換する
    let temp: u64 = text.trim().parse()?;
    let temp = temp as f64 / 1000.0;

    Ok(CpuTemp { temp })
}
