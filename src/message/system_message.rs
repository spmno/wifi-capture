use std::convert::TryInto;
use std::fmt;

use tracing::info;

use super::message::{Message, MessageError};

// SystemMessage 结构体
#[derive(Debug, Clone, PartialEq)]
pub struct SystemMessage {
    // 起始字节1 (1字节)
    pub coordinate_system: u8,     // 坐标系类型 (7位)
    pub reserved_bits: u8,         // 预留位 (6-5位)
    pub classification_region: u8, // 等级分类归属区域 (4-2位)
    pub station_type: u8,          // 控制站位置类型 (1-0位)

    // 起始字节2 (4字节)
    pub latitude: i32,             // 控制站纬度 (小端序)

    // 起始字节6 (4字节)
    pub longitude: i32,             // 控制站经度 (小端序)

    // 可选字段
    pub operation_count: Option<u16>, // 运行区域计数 (小端序)
    pub operation_radius: Option<u8>, // 运行区域半径 (*10)
    pub altitude_upper: Option<u16>,  // 运行区域高度上限 (几何高度, 小端序)
    pub altitude_lower: Option<u16>,  // 运行区域高度下限 (几何高度, 小端序)

    // 起始字节17 (1字节)
    pub ua_category: u8,           // UA运行类别

    // 起始字节18 (1字节)
    pub ua_level: u8,              // UA等级

    // 起始字节19 (2字节)
    pub station_altitude: u16,     // 控制站高度 (小端序)

    // 可选字段
    pub timestamp: Option<u32>,     // 时间戳 (Unix时间, 秒)
    pub reserved: Option<u8>,       // 预留
}

impl SystemMessage {
    pub const MESSAGE_TYPE: u8 = 0x04;
    const EXPECTED_LENGTH: usize = 24;
}


// 实现 Message trait
impl Message for SystemMessage {
    fn from_bytes(data: &[u8]) -> Result<Self, MessageError> {
        
        if data.len() < Self::EXPECTED_LENGTH {
            return Err(MessageError::InsufficientLength(
                Self::EXPECTED_LENGTH,
                data.len()
            ));
        }

        // 解析起始字节1
        let byte0 = data[0];
        let coordinate_system = (byte0 >> 5) & 0x07; // 取bit7-5
        let reserved_bits = (byte0 >> 3) & 0x03;    // 取bit6-5
        let classification_region = (byte0 >> 2) & 0x07; // 取bit4-2
        
        // 验证分类区域值
        if classification_region == 0 || classification_region > 3 {
            info!("class region = {}", classification_region);
            return Err(MessageError::UnknownMessageType(1));
        }
        
        let station_type = byte0 & 0x03; // 取bit1-0

        // 解析控制站纬度 (小端序)
        let latitude = i32::from_le_bytes(data[1..5].try_into()
            .map_err(|_| MessageError::InsufficientLength(5, data.len()))?);

        // 解析控制站经度 (小端序)
        let longitude = i32::from_le_bytes(data[5..9].try_into()
            .map_err(|_| MessageError::InsufficientLength(9, data.len()))?);

        // 处理可选字段（起始字节10）
        let mut offset = 9;
        let operation_count = if data.len() > offset + 1 {
            let value = u16::from_le_bytes([data[offset], data[offset+1]]);
            offset += 2;
            Some(value)
        } else {
            None
        };

        let operation_radius = if data.len() > offset {
            let value = data[offset];
            offset += 1;
            Some(value)
        } else {
            None
        };

        let altitude_upper = if data.len() > offset + 1 {
            let value = u16::from_le_bytes([data[offset], data[offset+1]]);
            offset += 2;
            Some(value)
        } else {
            None
        };

        let altitude_lower = if data.len() > offset + 1 {
            let value = u16::from_le_bytes([data[offset], data[offset+1]]);
            offset += 2;
            Some(value)
        } else {
            None
        };

        // 解析必送字段
        let ua_category = data[offset];
        offset += 1;
        
        let ua_level = data[offset];
        offset += 1;
        
        // 解析控制站高度
        let station_altitude = if data.len() > offset + 1 {
            u16::from_le_bytes([data[offset], data[offset+1]])
        } else {
            return Err(MessageError::InsufficientLength(offset + 2, data.len()));
        };
        offset += 2;

        // 处理可选尾部字段
        let timestamp = if data.len() > offset + 3 {
            let value = u32::from_le_bytes([
                data[offset], data[offset+1], data[offset+2], data[offset+3]
            ]);
            offset += 4;
            Some(value)
        } else {
            None
        };

        let reserved = if data.len() > offset {
            Some(data[offset])
        } else {
            None
        };

        Ok(Self {
            coordinate_system,
            reserved_bits,
            classification_region,
            station_type,
            latitude,
            longitude,
            operation_count,
            operation_radius,
            altitude_upper,
            altitude_lower,
            ua_category,
            ua_level,
            station_altitude,
            timestamp,
            reserved,
        })
    }

    fn print(&self) {
        println!("=== 系统消息 (SystemMessage) ===");
        println!("坐标系类型: {}", self.coordinate_system);
        println!("预留位: {:02b}", self.reserved_bits);
        println!("等级分类归属区域: {}", match self.classification_region {
            2 => "中国",
            3..=7 => "预留",
            _ => "未定义或无效",
        });
        println!("控制站位置类型: {}", self.station_type);
        println!("控制站纬度: {:.6}°", self.latitude as f64 * 1e-7);
        println!("控制站经度: {:.6}°", self.longitude as f64 * 1e-7);
        
        if let Some(count) = self.operation_count {
            println!("运行区域计数: {}", count);
        }
        if let Some(radius) = self.operation_radius {
            println!("运行区域半径: {} (实际: {} 米)", radius, radius as f32 * 10.0);
        }
        if let Some(alt_upper) = self.altitude_upper {
            println!("运行区域高度上限: {} (实际: {:.1} 米)", alt_upper, alt_upper as f32 * 0.1);
        }
        if let Some(alt_lower) = self.altitude_lower {
            println!("运行区域高度下限: {} (实际: {:.1} 米)", alt_lower, alt_lower as f32 * 0.1);
        }
        
        println!("UA运行类别: {}", self.ua_category);
        println!("UA等级: {}", self.ua_level);
        println!("控制站高度: {} (实际: {:.1} 米)", self.station_altitude, self.station_altitude as f32 * 0.1);
        
        if let Some(ts) = self.timestamp {
            // 实际应用中可将时间戳转换为可读时间
            println!("时间戳: {}", ts);
        }
        if let Some(res) = self.reserved {
            println!("预留字段: {:02X}", res);
        }
    }
}
