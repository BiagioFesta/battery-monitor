/*
    Copyright (C) 2022  Biagio Festa

    This program is free software: you can redistribute it and/or modify
    it under the terms of the GNU General Public License as published by
    the Free Software Foundation, either version 3 of the License, or
    (at your option) any later version.

    This program is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
    GNU General Public License for more details.

    You should have received a copy of the GNU General Public License
    along with this program.  If not, see <https://www.gnu.org/licenses/>.
*/

use anyhow::Context;
use anyhow::Result;
use battery::Battery;
use battery::Manager as BatteryManager;
use battery::State as BatteryState;
use notify_rust::Notification;
use notify_rust::Timeout as NotificationTimeout;
use notify_rust::Urgency as NotificationUrgency;
use std::thread::sleep;
use std::time::Duration;
use std::time::Instant;

#[derive(Clone, Copy, Eq, PartialEq)]
enum ServiceState {
    Normal,
    LowCapacity,
    CriticalCapacity,
}

impl Default for ServiceState {
    fn default() -> Self {
        Self::Normal
    }
}

impl ServiceState {
    const SECS_IN_MINUTE: u64 = 60;

    fn next_state(&self, battery: &Battery) -> ServiceState {
        match battery.state_of_charge().value {
            x if x < 0.1 => ServiceState::CriticalCapacity,
            x if x < 0.2 => ServiceState::LowCapacity,
            _ => ServiceState::Normal,
        }
    }

    fn renew_notification_time(&self) -> Duration {
        match self {
            ServiceState::Normal => Duration::MAX,
            ServiceState::LowCapacity => Duration::from_secs(10 * Self::SECS_IN_MINUTE),
            ServiceState::CriticalCapacity => Duration::from_secs(5 * Self::SECS_IN_MINUTE),
        }
    }

    fn send_notification(&self, battery: &Battery) {
        match self {
            ServiceState::Normal => (),
            ServiceState::LowCapacity => {
                Self::send_notification_raw(
                    NotificationUrgency::Normal,
                    NotificationTimeout::Default,
                    battery,
                );
            }
            ServiceState::CriticalCapacity => {
                Self::send_notification_raw(
                    NotificationUrgency::Critical,
                    NotificationTimeout::Never,
                    battery,
                );
            }
        }
    }

    fn send_notification_raw(
        urgency: NotificationUrgency,
        timeout: NotificationTimeout,
        battery: &Battery,
    ) {
        let capacity = battery.state_of_charge().value;

        let _ = Notification::new()
            .summary("Low Battery")
            .icon("battery")
            .urgency(urgency)
            .timeout(timeout)
            .body(&format!(
                "Battery capacity is {:.0}%{rem}",
                capacity * 100_f32,
                rem = battery
                    .time_to_empty()
                    .map(|time| format!(" (remaining: {:?})", time))
                    .unwrap_or_default(),
            ))
            .show();
    }
}

struct BatteryMonitor {
    battery_manager: BatteryManager,
    state: ServiceState,
}

impl BatteryMonitor {
    const REFRESH_TIME: Duration = Duration::from_secs(10);

    fn new() -> Result<Self> {
        let battery_manager = BatteryManager::new().context("Cannot create battery manager")?;
        let state = ServiceState::default();

        Ok(Self {
            battery_manager,
            state,
        })
    }

    fn run_service(mut self) -> Result<()> {
        let mut last_notification_time = Instant::now();

        loop {
            let batteries = self.query_batteries()?;

            let is_all_discharging = batteries.iter().all(|battery| {
                matches!(
                    battery.state(),
                    BatteryState::Discharging | BatteryState::Empty
                )
            });

            if is_all_discharging {
                let lower_battery = batteries.iter().min_by(|b1, b2| {
                    b1.state_of_charge()
                        .partial_cmp(&b2.state_of_charge())
                        .expect("Expect battery capacities are comparable")
                });

                if let Some(lower_battery) = lower_battery {
                    let next_state = self.state.next_state(lower_battery);

                    if next_state != self.state
                        || last_notification_time.elapsed() > self.state.renew_notification_time()
                    {
                        next_state.send_notification(lower_battery);
                        last_notification_time = Instant::now();
                    }

                    self.state = next_state;
                }
            } else {
                self.state = ServiceState::Normal;
            }

            sleep(Self::REFRESH_TIME);
        }
    }

    fn query_batteries(&self) -> Result<Vec<Battery>> {
        self.battery_manager
            .batteries()
            .context("Cannot retrieve batteries information")?
            .collect::<battery::Result<Vec<_>>>()
            .context("Battery information is missing")
    }
}

fn main() -> Result<()> {
    BatteryMonitor::new()?.run_service()
}
