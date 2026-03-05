use std::collections::HashMap;
use std::error::Error;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use telltale_core::{Event, Platform, Rule};

use crate::app;

use super::EventSource;

pub struct SimulatedSource {
    interval: Duration,
    count: u64,
    rules: Vec<Rule>,
    rng_state: u64,
    disk_index: usize,
    service_index: usize,
    host_index: usize,
    process_index: usize,
    volume_index: usize,
}

impl SimulatedSource {
    pub fn new(interval: Duration, count: u64) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let rules = app::rules_for_current_os();
        if rules.is_empty() {
            return Err(format!("no rules available for {}", std::env::consts::OS).into());
        }

        let rng_state = match SystemTime::now().duration_since(UNIX_EPOCH) {
            Ok(duration) => duration.as_nanos() as u64,
            Err(_) => 0x9E37_79B9_7F4A_7C15,
        };

        Ok(Self {
            interval,
            count,
            rules,
            rng_state,
            disk_index: 0,
            service_index: 0,
            host_index: 0,
            process_index: 0,
            volume_index: 0,
        })
    }

    fn next_rule_index(&mut self) -> usize {
        self.rng_state = self
            .rng_state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1);
        (self.rng_state as usize) % self.rules.len()
    }

    fn next_disk(&mut self) -> &'static str {
        const DISKS: [&str; 3] = ["sda", "sdb", "nvme0n1"];
        let value = DISKS[self.disk_index % DISKS.len()];
        self.disk_index = self.disk_index.wrapping_add(1);
        value
    }

    fn next_service(&mut self) -> &'static str {
        const SERVICES: [&str; 3] = ["ssh.service", "nginx.service", "docker.service"];
        let value = SERVICES[self.service_index % SERVICES.len()];
        self.service_index = self.service_index.wrapping_add(1);
        value
    }

    fn next_host(&mut self) -> &'static str {
        const HOSTS: [&str; 3] = ["workstation-01", "build-node-02", "media-server-03"];
        let value = HOSTS[self.host_index % HOSTS.len()];
        self.host_index = self.host_index.wrapping_add(1);
        value
    }

    fn next_process(&mut self) -> &'static str {
        const PROCESSES: [&str; 4] = ["postgres", "code-server", "prometheus", "redis-server"];
        let value = PROCESSES[self.process_index % PROCESSES.len()];
        self.process_index = self.process_index.wrapping_add(1);
        value
    }

    fn next_volume(&mut self) -> &'static str {
        const VOLUMES: [&str; 3] = ["C:", "D:", "E:"];
        let value = VOLUMES[self.volume_index % VOLUMES.len()];
        self.volume_index = self.volume_index.wrapping_add(1);
        value
    }

    fn simulated_event_for_rule(&mut self, rule_id: &str, platform: Platform) -> Event {
        match rule_id {
            "win.disk.bad_block" => self.windows_disk_bad_block(),
            "win.ntfs.corruption" => self.windows_ntfs_corruption(),
            "win.system.unexpected_shutdown" => self.windows_unexpected_shutdown(),
            "win.whea.hardware_error" => self.windows_whea_hardware_error(),
            "win.bugcheck.summary" => self.windows_bugcheck(),
            "linux.oom_killer" => self.linux_oom_killer(),
            "linux.ext4_error" => self.linux_ext4_error(),
            "linux.auth_failure" => self.linux_auth_failure(),
            "linux.systemd_service_failure" => self.linux_systemd_service_failure(),
            _ => self.fallback_event(platform, rule_id),
        }
    }

    fn windows_disk_bad_block(&mut self) -> Event {
        let disk = self.next_disk();
        let host = self.next_host();
        let mut metadata = HashMap::new();
        metadata.insert("device".to_string(), disk.to_string());
        metadata.insert("disk".to_string(), disk.to_string());
        metadata.insert("computer".to_string(), host.to_string());
        metadata.insert("entity".to_string(), disk.to_string());

        Event {
            timestamp: SystemTime::now(),
            platform: Platform::Windows,
            source: "Disk".to_string(),
            event_id: Some(7),
            message: format!("The device, {disk}, has a bad block reported by the storage stack."),
            metadata,
        }
    }

    fn windows_ntfs_corruption(&mut self) -> Event {
        let volume = self.next_volume();
        let host = self.next_host();
        let mut metadata = HashMap::new();
        metadata.insert("volume".to_string(), volume.to_string());
        metadata.insert("drive".to_string(), volume.to_string());
        metadata.insert("computer".to_string(), host.to_string());
        metadata.insert("entity".to_string(), volume.to_string());

        Event {
            timestamp: SystemTime::now(),
            platform: Platform::Windows,
            source: "Ntfs".to_string(),
            event_id: Some(55),
            message: format!(
                "The file system structure on volume {volume} is corrupt and unusable."
            ),
            metadata,
        }
    }

    fn windows_unexpected_shutdown(&mut self) -> Event {
        let host = self.next_host();
        let mut metadata = HashMap::new();
        metadata.insert("computer".to_string(), host.to_string());
        metadata.insert("entity".to_string(), host.to_string());

        Event {
            timestamp: SystemTime::now(),
            platform: Platform::Windows,
            source: "EventLog".to_string(),
            event_id: Some(6008),
            message: "The previous system shutdown was unexpected.".to_string(),
            metadata,
        }
    }

    fn windows_whea_hardware_error(&mut self) -> Event {
        const EVENT_IDS: [u64; 4] = [17, 18, 19, 20];
        let index = self.host_index % EVENT_IDS.len();
        let event_id = EVENT_IDS[index];
        let cpu = format!("CPU{}", self.host_index % 4);
        let bank = format!("Bank {}", self.host_index % 3);
        let host = self.next_host();

        let mut metadata = HashMap::new();
        metadata.insert("processor".to_string(), cpu.clone());
        metadata.insert("bank".to_string(), bank.clone());
        metadata.insert("computer".to_string(), host.to_string());
        metadata.insert("entity".to_string(), cpu.clone());

        Event {
            timestamp: SystemTime::now(),
            platform: Platform::Windows,
            source: "Microsoft-Windows-WHEA-Logger".to_string(),
            event_id: Some(event_id),
            message: format!("A corrected hardware error has occurred on {cpu} ({bank})."),
            metadata,
        }
    }

    fn windows_bugcheck(&mut self) -> Event {
        const BUGCHECK_CODES: [&str; 4] = ["0x0000007E", "0x00000124", "0x00000050", "0x00000139"];
        let code = BUGCHECK_CODES[self.process_index % BUGCHECK_CODES.len()];
        let host = self.next_host();
        let mut metadata = HashMap::new();
        metadata.insert("bugcheck_code".to_string(), code.to_string());
        metadata.insert("computer".to_string(), host.to_string());
        metadata.insert("entity".to_string(), code.to_string());

        Event {
            timestamp: SystemTime::now(),
            platform: Platform::Windows,
            source: "BugCheck".to_string(),
            event_id: Some(1001),
            message: format!("The computer has rebooted from a bugcheck. Bugcheck code: {code}."),
            metadata,
        }
    }

    fn linux_oom_killer(&mut self) -> Event {
        let process = self.next_process();
        let host = self.next_host();
        let mut metadata = HashMap::new();
        metadata.insert("entity".to_string(), process.to_string());
        metadata.insert("host".to_string(), host.to_string());

        Event {
            timestamp: SystemTime::now(),
            platform: Platform::Linux,
            source: "kernel".to_string(),
            event_id: None,
            message: format!(
                "Out of memory: Killed process 8421 ({process}) total-vm:512000kB anon-rss:220000kB"
            ),
            metadata,
        }
    }

    fn linux_ext4_error(&mut self) -> Event {
        let disk = self.next_disk();
        let host = self.next_host();
        let mut metadata = HashMap::new();
        metadata.insert("entity".to_string(), disk.to_string());
        metadata.insert("host".to_string(), host.to_string());

        Event {
            timestamp: SystemTime::now(),
            platform: Platform::Linux,
            source: "kernel".to_string(),
            event_id: None,
            message: format!(
                "EXT4-fs error (device {disk}): ext4_find_entry: inode #120117: comm rsync: reading directory lblock 0"
            ),
            metadata,
        }
    }

    fn linux_auth_failure(&mut self) -> Event {
        let service = self.next_service();
        let host = self.next_host();
        let mut metadata = HashMap::new();
        metadata.insert("entity".to_string(), service.to_string());
        metadata.insert("host".to_string(), host.to_string());

        Event {
            timestamp: SystemTime::now(),
            platform: Platform::Linux,
            source: "sshd".to_string(),
            event_id: None,
            message: "Failed password for invalid user admin from 203.0.113.24 port 53122 ssh2"
                .to_string(),
            metadata,
        }
    }

    fn linux_systemd_service_failure(&mut self) -> Event {
        let service = self.next_service();
        let host = self.next_host();
        let mut metadata = HashMap::new();
        metadata.insert("entity".to_string(), service.to_string());
        metadata.insert("host".to_string(), host.to_string());

        Event {
            timestamp: SystemTime::now(),
            platform: Platform::Linux,
            source: "systemd".to_string(),
            event_id: None,
            message: format!("{service}: Main process exited, failed with result 'exit-code'."),
            metadata,
        }
    }

    fn fallback_event(&self, platform: Platform, rule_id: &str) -> Event {
        let mut metadata = HashMap::new();
        metadata.insert("entity".to_string(), "simulated".to_string());

        Event {
            timestamp: SystemTime::now(),
            platform,
            source: "simulated".to_string(),
            event_id: None,
            message: format!("Synthetic event generated for unsupported rule {rule_id}"),
            metadata,
        }
    }
}

impl EventSource for SimulatedSource {
    fn name(&self) -> &'static str {
        "simulated"
    }

    fn watch(&mut self, sender: mpsc::Sender<Event>) -> Result<(), Box<dyn Error + Send + Sync>> {
        let mut produced = 0u64;

        loop {
            if self.count != 0 && produced >= self.count {
                return Ok(());
            }

            let rule_index = self.next_rule_index();
            let rule_id = self.rules[rule_index].id;
            let rule_platform = self.rules[rule_index].platform;
            let event = self.simulated_event_for_rule(rule_id, rule_platform);

            if !self.rules[rule_index].matches(&event) {
                return Err(
                    format!("simulated event did not match selected rule {rule_id}").into(),
                );
            }

            if sender.send(event).is_err() {
                return Ok(());
            }

            produced = produced.saturating_add(1);

            if self.count == 0 || produced < self.count {
                thread::sleep(self.interval);
            }
        }
    }
}
