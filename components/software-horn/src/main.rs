/*******************************************************************************
* Copyright (c) 2024 Contributors to the Eclipse Foundation
*
* See the NOTICE file(s) distributed with this work for additional
* information regarding copyright ownership.
*
* This program and the accompanying materials are made available under the
* terms of the Eclipse Public License 2.0 which is available at
* http://www.eclipse.org/legal/epl-2.0
*
* SPDX-License-Identifier: EPL-2.0
*******************************************************************************/

use clap::Parser;
use env_logger::Env;
use log::{debug, error, info, warn};
use zenoh::pubsub::Publisher;
use zenoh::sample::Sample;
use zenoh::Config;

#[derive(clap::Parser)]
pub struct Args {
    #[arg(short, long, env = "ZENOH_CONFIG")]
    /// A Zenoh configuration file.
    config: String,
    #[arg(short, long, default_value = "true", env = "IS_SOUND_ENABLED")]
    sound: bool,
}

impl Args {
    pub fn get_zenoh_config(&self) -> Result<Config, Box<dyn std::error::Error>> {
        // Load the config from file path
        zenoh::config::Config::from_file(&self.config).map_err(|e| e as Box<dyn std::error::Error>)
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
    let args = Args::parse();
    let zenoh_config = args.get_zenoh_config()?;
    info!("Starting the software horn connected over Eclipse Zenoh");

    let horn_keyexpr = String::from("Vehicle/Body/Horn/IsActive");

    let session = zenoh::open(zenoh_config)
        .await
        .map_err(|e| e as Box<dyn std::error::Error>)?;

    let subscriber = session
        .declare_subscriber(&horn_keyexpr)
        .await
        .map_err(|e| e as Box<dyn std::error::Error>)?;

    let publisher = session
        .declare_publisher(&horn_keyexpr)
        .await
        .map_err(|e| e as Box<dyn std::error::Error>)?;
    debug!("Waiting for messages on topic: {}", &horn_keyexpr);

    while let Ok(sample) = subscriber.recv_async().await {
        if contains_target_value(&sample) {
            if let Some(status) = payload_as_bool(&sample) {
                info!("{}activating horn signal", if status { "" } else { "de" });
                publish_current_status(&publisher, status).await;
            } else {
                error!("Payload from Zenoh message is not a boolean");
            }
        }
    }

    Ok(())
}

async fn publish_current_status(publisher: &Publisher<'_>, status: bool) {
    if let Err(e) = publisher
        .put(status.to_string())
        .attachment("currentValue")
        .await
    {
        warn!("failed to publish current status: {e}");
    }
}

fn contains_target_value(sample: &Sample) -> bool {
    sample
        .attachment()
        .and_then(|a| a.try_to_string().map(|v| v.to_string()).ok())
        .map_or(false, |v| v == "targetValue")
}

fn payload_as_bool(sample: &Sample) -> Option<bool> {
    let zbuf = sample.payload();
    zbuf.try_to_string()
        .ok()
        .map(|v| v.to_string())
        .map(|s| s.parse::<bool>().unwrap_or(false))
}
