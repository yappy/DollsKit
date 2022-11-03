use crate::sys::config;
use crate::sys::taskserver::Control;
use super::SystemModule;
use anyhow::{Result, ensure};
use chrono::NaiveTime;
use log::info;
use serde::{Serialize, Deserialize};

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

        let jsobj = config::get_object(&["health"]);
        ensure!(jsobj != None, "Config not found: health");
        let jsobj = jsobj.unwrap();
        let config: HealthConfig = serde_json::from_value(jsobj)?;

        Ok(Health {
            config,
            wakeup_list,
        })
    }

    async fn health_task_entry(ctrl: Control) -> Result<()> {
        unimplemented!();
        //let mut health = ctrl.sysmods().health.write().await;
        //health.health_task_entry(&ctrl).await
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
