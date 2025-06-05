use message::{message::{Message, MessageError}, AnyMessage};
use tracing::{info, error};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt};
use tracing_appender::{non_blocking, rolling::{self}};
use pnet::datalink::{self, interfaces, Channel, NetworkInterface};
use libwifi::{frame::{self, Beacon}, parse_frame, Frame};
use chrono::Local;
use std::ops::Range;

pub mod wifi;
pub mod message;
pub mod upload_data;


use crate::message::base_message::BaseMessage;
use crate::message::position_vector_message::PositionVectorMessage;
use crate::upload_data::UploadData;

fn get_wifi_devices() -> Vec<NetworkInterface> {
 let interfaces = interfaces();
    let mut wifi_devices = Vec::new();

    info!("Available WiFi network devices:");
    for interface in interfaces {
        // 根据操作系统调整过滤条件
        if interface.name.contains("wl") || interface.name.contains("wlan") {
            info!("Name: {}, MAC: {:?}", interface.name, interface.mac);
            wifi_devices.push(interface);
        }
    }
    wifi_devices
}

fn capture_wifi_channel(interface: NetworkInterface)  {
let (mut tx, mut rx) = match datalink::channel(&interface, Default::default()) {
        Ok(Channel::Ethernet(tx, rx)) => (tx, rx),
        Ok(_) => {
            error!("Unsupported channel type");
            return;
        }
        Err(e) => {
            error!("Failed to create channel: {}", e);
            return;
        }
    };

    info!("Capturing on {}", interface.name);
    
    loop {
        match rx.next() {
            Ok(packet) => {
                process_packet(packet);
                let current_time = Local::now().format("%H:%M:%S").to_string();
                info!("当前时间: {}", current_time);
            }
            Err(e) => {
                error!("Error reading packet: {}", e);
                break;
            }
        }
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
            //info!("Got frame: {frame:?}");
            if let Frame::Beacon(beacon) = frame {
                info!("this is the beacon frame: {:?}", beacon);
                info!("vendor info: {:?}", beacon.station_info.vendor_specific);
                if (beacon.station_info.vendor_specific[0].element_id == 221) && (beacon.station_info.vendor_specific[0].oui_type == 13) {
                    let mut upload_data = UploadData { rid: String::from(""), longitude: 0, latitude: 0 };
                    let ssid = beacon.station_info.ssid();
                    let vendor_data = &beacon.station_info.vendor_specific[0].data;
                    info!("this is the openid element, ssid: {:?}, total len: {}, pack count: {}, pack size: {}", ssid, vendor_data[0], vendor_data[3], vendor_data[2]);
                    let count = vendor_data[3];
                    for i in 0..count {
                        
                        let range: Range<usize> = ((25*i+4) as usize)..((25*i+29) as usize);
                        info!("i = {}, range:{:?}", i, range);
                        let pack = &vendor_data[range];
                        let message = AnyMessage::from_bytes(pack).unwrap();
                        match message {
                            AnyMessage::Base(bm) => {
                                bm.print();
                                upload_data.rid = bm.uas_id;
                            }, 
                            AnyMessage::PositionVector(pvm) => {
                                pvm.print();
                                upload_data.longitude = pvm.longitude;
                                upload_data.latitude = pvm.latitude;
                            },
                            AnyMessage::System(sm) => {
                                sm.print();
                            }
                        }
                    }
                }
            } else {
                info!("not beacon frame.");
            }
        }
        Err(err) => {
            error!("Error during parsing : {err:?}");
        }
    }
}

fn create_special_message(data: &[u8]) -> Result<Box<dyn Message>, MessageError> {
    let message_type = (data[0] >> 4) & 0x0f;
    let content = &data[1..];
    match message_type {
        BaseMessage::MESSAGE_TYPE => {
            let message = BaseMessage::from_bytes(content);
            match message {
                Ok(message) => {
                    return Ok(Box::new(message));
                },
                Err(err) => {
                    error!("base error: {}", err);
                    return  Err(err);
                }
            }
        },
        PositionVectorMessage::MESSAGE_TYPE =>{
            let message = PositionVectorMessage::from_bytes(content);
            match message {
                Ok(message) => {
                    return Ok(Box::new(message));
                },
                Err(err) => {
                    error!("base error: {}", err);
                    return  Err(err);
                }
            }
        }
        _ => {
            return Err(MessageError::UnknownMessageType(0));
        }
    }
}


fn process_packet(packet: &[u8]) {
    if packet.len() < 100 {
        return;
    }
    //let data = packet.data;
    let (radiotap, remaining) = parse_radiotap(packet);
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


#[cfg(test)]
mod tests {
    // 注意这个惯用法：在 tests 模块中，从外部作用域导入所有名字。
    use super::*;

    #[test]
    fn test_process_packet() {
        let file_appender = rolling::daily("logs", "capture.log");
    let (non_blocking_appender, _guard) = non_blocking(file_appender);
    let file_layer = fmt::layer()
        .with_ansi(false)
        .with_writer(non_blocking_appender);

    let console_subscriber = fmt::layer().with_writer(std::io::stdout);

    tracing_subscriber::registry().with(console_subscriber).with(file_layer).init();
        let packet = vec![0x00, 0x00, 0x26, 0x00, 0x2f, 0x40, 0x00, 0xa0,  0x20, 0x08, 0x00, 0xa0, 0x20, 0x08, 0x00, 0x00,
                                   0x74, 0x71, 0xf3, 0x0b, 0x00, 0x00, 0x00, 0x00,  0x10, 0x0c, 0x85, 0x09, 0xc0, 0x00, 0x10, 0x00,
                                   0x00, 0x00, 0xc4, 0x00, 0x10, 0x01, 0x80, 0x00,  0x00, 0x00, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff,
                                   0xe4, 0x7a, 0x2c, 0x24, 0x3d, 0x26, 0xe4, 0x7a,  0x2c, 0x24, 0x3d, 0x26, 0x00, 0x00, 0x80, 0x84,
                                   0x00, 0x05, 0x00, 0x00, 0x00, 0x00, 0xa0, 0x00,  0x20, 0x04, 0x00, 0x18, 0x52, 0x49, 0x44, 0x2d,
                                   0x31, 0x35, 0x38, 0x31, 0x46, 0x37, 0x46, 0x56,  0x43, 0x32, 0x35, 0x31, 0x41, 0x30, 0x30, 0x43,
                                   0x51, 0x32, 0x35, 0x43, 0xdd, 0x53, 0xfa, 0x0b,  0xbc, 0x0d, 0x75, 0xf1, 0x19, 0x03, 0x01, 0x12,
                                   0x31, 0x35, 0x38, 0x31, 0x46, 0x37, 0x46, 0x56,  0x43, 0x32, 0x35, 0x31, 0x41, 0x30, 0x30, 0x43,
                                   0x51, 0x32, 0x35, 0x43, 0x00, 0x00, 0x00, 0x11,  0x22, 0xb5, 0x00, 0x00, 0xfd, 0x1d, 0xdd, 0x18,
                                   0xe3, 0x39, 0x9a, 0x49, 0xf2, 0x08, 0x48, 0x08,  0xd2, 0x07, 0x3b, 0x04, 0xee, 0x13, 0x0a, 0x00,
                                   0x41, 0x08, 0x00, 0x1e, 0xdd, 0x18, 0x00, 0x3a,  0x9a, 0x49, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00,
                                   0x00, 0x01, 0x46, 0x08, 0xae, 0xce, 0xd1, 0x0b,  0x00, 0xb6, 0xba, 0x45, 0xe7];
        info!("start process packet.");
        process_packet(&packet);
        assert_eq!(4, 3);
    }

}