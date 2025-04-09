use pcap::{Capture, Device, Packet};
use serde::Serialize;
use std::collections::HashMap;
use libwifi::{frame::{self, Beacon}, parse_frame, Frame};

pub mod wifi;


fn get_wifi_devices() -> Vec<Device> {
    let devices = Device::list().unwrap();
    let mut wifi_devices = Vec::new();

    println!("Available WiFi network devices:");
    for device in devices {
        if device.name.contains("wl") {
            println!("Name: {}, Description: {:?}", device.name, device.desc);
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

    println!("Capturing on device");

    while let Ok(packet) = cap.next_packet() {
        println!("received packet! {:?}", packet);
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
            println!("Got frame: {frame:?}");
            if let Frame::Beacon(beacon) = frame {
                println!("this is the beacon frame: {:?}", beacon);
            } else {
                println!("not beacon frame.");
            }
        }
        Err(err) => {
            println!("Error during parsing : {err:?}");
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
    let wifi_devices = get_wifi_devices();
    if !wifi_devices.is_empty() {
        capture_wifi_channel(wifi_devices.first().unwrap().clone());
    }
}
