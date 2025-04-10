use tracing::{info, error};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, Layer};
use tracing_appender::{non_blocking, rolling::{self}};
use pcap::{Capture, Device, Packet};
use libwifi::{frame::{self, Beacon}, parse_frame, Frame};


pub mod wifi;


fn get_wifi_devices() -> Vec<Device> {
    let devices = Device::list().unwrap();
    let mut wifi_devices = Vec::new();

    info!("Available WiFi network devices:");
    for device in devices {
        if device.name.contains("wl") {
            info!("Name: {}, Description: {:?}", device.name, device.desc);
            wifi_devices.push(device);
        }
    }
    wifi_devices
}

fn capture_wifi_channel(device: Device)  {
    let mut cap = Capture::from_device(device).unwrap()
        .promisc(true)
        .snaplen(65535)
        .open().unwrap();

    info!("Capturing on device");

    while let Ok(packet) = cap.next_packet() {
        info!("received packet! {:?}", packet);
        process_packet(packet);
    }
}

struct RadiotapHeader {
    signal: f32,
    rate: f32,
    channel_freq: u16,
}

fn parse_80211_mgt(data: &[u8]) {
    match parse_frame(data, false) {
        Ok(frame) => {
            info!("Got frame: {frame:?}");
            if let Frame::Beacon(beacon) = frame {
                info!("this is the beacon frame: {:?}", beacon);
                info!("vendor info: {:?}", beacon.station_info.vendor_specific);
            } else {
                info!("not beacon frame.");
            }
        }
        Err(err) => {
            error!("Error during parsing : {err:?}");
        }
    }
}


fn process_packet(packet: Packet) {
    if packet.len() < 200 {
        return;
    }
    let data = packet.data;
    let (radiotap, remaining) = parse_radiotap(data);
    parse_80211_mgt(remaining);
}

fn parse_radiotap(data: &[u8]) -> (RadiotapHeader, &[u8]) {
    let mut offset = 0;
    let header_len = data[2] as usize;
    
    let mut signal = 0.0;
    let mut rate = 0.0;
    let mut channel_freq = 0;

    while offset < header_len {
        let field_type = data[offset];
        offset += 1;
        
        match field_type {
            0x03 => { // Signal
                signal = data[offset] as i8 as f32;
                offset += 1;
            }
            0x02 => { // Rate
                rate = (data[offset] as f32) * 0.5;
                offset += 1;
            }
            0x12 => { // Channel
                channel_freq = u16::from_le_bytes([data[offset], data[offset+1]]);
                offset += 4;
            }
            _ => break,
        }
    }

    (RadiotapHeader { signal, rate, channel_freq }, &data[header_len..])
}

fn main() {
    let file_appender = rolling::daily("logs", "capture.log");
    //let file_appender = BasicRollingFileAppender::new("./logs", RollingConditionBasic::new().daily(), MAX_FILE_COUNT).unwrap();
    let (non_blocking_appender, _guard) = non_blocking(file_appender);
    let file_layer = fmt::layer()
        .with_ansi(false)
        .with_writer(non_blocking_appender);

    let console_subscriber = fmt::layer().with_writer(std::io::stdout);

    tracing_subscriber::registry().with(console_subscriber).with(file_layer).init();
    let wifi_devices = get_wifi_devices();
    if !wifi_devices.is_empty() {
        capture_wifi_channel(wifi_devices.first().unwrap().clone());
    }
}
