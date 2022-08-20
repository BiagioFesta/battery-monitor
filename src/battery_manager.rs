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

use anyhow::anyhow;
use anyhow::Context;
use anyhow::Result;
use std::fmt::Debug;
use std::time::Duration;
use zbus::Connection as ZbusConnection;

pub struct BatteryManager {
    zbus_connection: ZbusConnection,
}

impl BatteryManager {
    const DEVICE_TYPE_BATTERY: u32 = 2;

    pub async fn new() -> Result<Self> {
        let zbus_connection = ZbusConnection::system()
            .await
            .context("Cannot establish a dbus connection")?;

        Ok(Self { zbus_connection })
    }

    pub async fn get_batteries(&self) -> Result<Vec<Battery>> {
        let upower = dbus_interfaces::UPowerProxy::new(&self.zbus_connection)
            .await
            .context("Cannot query upower interface service")?;

        let dev_paths = upower
            .enumerate_devices()
            .await
            .context("Cannot enumerate devices")?;

        let mut batteries = Vec::with_capacity(dev_paths.len());

        for dev_path in dev_paths {
            let device = dbus_interfaces::DeviceProxy::builder(&self.zbus_connection)
                .path(dev_path)
                .context("Invalid device path")?
                .build()
                .await
                .context("Cannot query upower device interface")?;

            match device.type_().await {
                Ok(device_type) => {
                    if device_type == Self::DEVICE_TYPE_BATTERY {
                        batteries.push(Battery::new(&device).await?);
                    }
                }
                Err(error) => {
                    eprintln!(
                        "{:?}",
                        anyhow!(error).context("Cannot detect type of device")
                    );
                }
            }
        }

        Ok(batteries)
    }
}

#[derive(Debug)]
pub struct Battery {
    state: BatteryState,
    percentage: f64,
    time_to_empty: Duration,
}

impl Battery {
    async fn new(device: &dbus_interfaces::DeviceProxy<'_>) -> Result<Battery> {
        let state = BatteryState::from_upower_code(
            device
                .state()
                .await
                .context("Cannot retrieve 'state' information")?,
        );

        let percentage = device
            .percentage()
            .await
            .context("Cannot retrieve 'percentage' information")?;

        let time_to_empty = Duration::from_secs(
            device
                .time_to_empty()
                .await
                .context("Cannot retrieve 'time_to_empty' information")? as u64,
        );

        Ok(Self {
            state,
            percentage,
            time_to_empty,
        })
    }

    pub fn state(&self) -> BatteryState {
        self.state
    }

    pub fn percentage(&self) -> f64 {
        self.percentage
    }

    pub fn time_to_empty(&self) -> Duration {
        self.time_to_empty
    }
}

#[derive(Debug, Clone, Copy)]
pub enum BatteryState {
    Unknown,
    Charging,
    Discharging,
    Empty,
    FullyCharged,
    PendingCharge,
    PendingDischarge,
}

impl BatteryState {
    fn from_upower_code(code: u32) -> Self {
        match code {
            0 => Self::Unknown,
            1 => Self::Charging,
            2 => Self::Discharging,
            3 => Self::Empty,
            4 => Self::FullyCharged,
            5 => Self::PendingCharge,
            6 => Self::PendingDischarge,
            _ => unreachable!("Invalid code"),
        }
    }
}

mod dbus_interfaces {
    #[zbus::dbus_proxy(interface = "org.freedesktop.UPower")]
    trait UPower {
        fn enumerate_devices(&self) -> zbus::Result<Vec<zbus::zvariant::OwnedObjectPath>>;
    }

    #[zbus::dbus_proxy(
        interface = "org.freedesktop.UPower.Device",
        default_service = "org.freedesktop.UPower"
    )]
    trait Device {
        #[dbus_proxy(property, name = "Type")]
        fn type_(&self) -> zbus::Result<u32>;

        #[dbus_proxy(property)]
        fn state(&self) -> zbus::Result<u32>;

        #[dbus_proxy(property)]
        fn percentage(&self) -> zbus::Result<f64>;

        #[dbus_proxy(property)]
        fn time_to_empty(&self) -> zbus::Result<i64>;

        fn refresh(&self) -> zbus::Result<()>;
    }
}
