use std::time::Duration;

use esp_idf_hal::sys::EspError;
use esp_idf_svc::{eventloop::EspSystemEventLoop, hal::prelude::Peripherals, mqtt::client::{EspMqttClient, EspMqttConnection, MqttClientConfiguration}};
use log::*;
use wifi::wifi;

#[derive(Debug)]
#[toml_cfg::toml_config]
pub struct Config {
    #[default("")]
    ssid: &'static str,
    #[default("")]
    password: &'static str,
}

const MQTT_URL: &str = "mqtt://broker.emqx.io:1883";
const MQTT_CLIENT_ID: &str = "esp-mqtt-demo";
const MQTT_TOPIC: &str = "esp-mqtt-demo";

fn main() -> anyhow::Result<()> {
    // It is necessary to call this function once. Otherwise some patches to the runtime
    // implemented by esp-idf-sys might not link properly. See https://github.com/esp-rs/esp-idf-template/issues/71
    esp_idf_svc::sys::link_patches();

    // Bind the log crate to the ESP Logging facilities
    esp_idf_svc::log::EspLogger::initialize_default();

    log::info!("Hello, world!");

    let peripheral = Peripherals::take()?;
    let sysloop = EspSystemEventLoop::take()?;

    let app_config = CONFIG;

    dbg!(CONFIG);

    let _wifi = wifi(
        app_config.ssid,
        app_config.password,
        peripheral.modem,
        sysloop,
    )?;

    let (mut client, mut conn) = mqtt_create(MQTT_URL, MQTT_CLIENT_ID).unwrap();

    run(&mut client, &mut conn, MQTT_TOPIC).unwrap();

    Ok(())
}

fn run(
    client: &mut EspMqttClient<'_>,
    connection: &mut EspMqttConnection,
    topic: &str,
) -> Result<(), EspError> {
    std::thread::scope(|s| {
        info!("About to start the MQTT client");

        std::thread::Builder::new()
            .stack_size(6000)
            .spawn_scoped(s, move || {
                info!("MQTT listening for messages");

                while let Ok(event) = connection.next() {
                    info!("[Queue] Event: {}", event.payload());
                }

                info!("Connection closed");
            })
            .unwrap();

        loop {
            if let Err(e) = client.subscribe(topic, esp_idf_svc::mqtt::client::QoS::AtMostOnce) {
                error!("Failed to subscribe to topic \"{topic}\": {e}, retrying...");

                std::thread::sleep(Duration::from_millis(500));

                continue;
            }
            
            info!("Subscribed to topic \"{topic}\"");

            std::thread::sleep(Duration::from_millis(500));

            let payload = "Hello from esp-mqtt-demo!";

            loop {
                client.enqueue(topic, esp_idf_svc::mqtt::client::QoS::AtMostOnce, false, payload.as_bytes())?;

                info!("Published \"{payload}\"");

                let sleep_secs = 2;

                info!("Now sleeping for {sleep_secs}...");
                std::thread::sleep(Duration::from_secs(sleep_secs));
            }
        }
    })
}

fn mqtt_create(
    url: &str,
    client_id: &str,
) -> Result<(EspMqttClient<'static>, EspMqttConnection), EspError> {
    let (mqtt_client, mqtt_conn) = EspMqttClient::new(
        url, 
        &MqttClientConfiguration {
            client_id: Some(client_id),
            ..Default::default()
        }
    )?;

    Ok((mqtt_client, mqtt_conn))
}