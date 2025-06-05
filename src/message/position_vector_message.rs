use std::convert::TryInto;
use std::fmt;

use super::message::{Message, MessageError};

#[derive(Debug, Clone, PartialEq)]
pub struct PositionVectorMessage {
    // 第1字节 (运行状态和标志位)
    pub run_status: u8,         // 运行状态 (7-4位)
    pub reserved_flag: bool,     // 预留标志位 (3位)
    pub height_type: u8,        // 高度类型位 (2位) - 0-3
    pub track_direction: bool,   // 航迹角 E/W 方向标志 (1位)
    pub speed_multiplier: bool,  // 速度乘数 (0位)

    // 第2-4字节
    pub track_angle: u8,        // 航迹角 (1字节)
    pub ground_speed: i8,       // 地速 (1字节, 有正负)
    pub vertical_speed: i8,     // 垂直速度 (1字节, 有正负, 可选)

    // 第5-18字节
    pub latitude: i32,           // 纬度 (4字节小端序)
    pub longitude: i32,          // 经度 (4字节小端序)
    pub pressure_altitude: i16, // 气压高度 (2字节小端序, 可选)
    pub geometric_altitude: i16, // 几何高度 (2字节小端序, 可选)
    pub ground_altitude: i16,    // 距地高度 (2字节小端序)

    // 第19-22字节
    pub vertical_accuracy: u8,   // 垂直精度 (7-4位, 4 bits)
    pub horizontal_accuracy: u8, // 水平精度 (3-0位, 4 bits)
    pub speed_accuracy: u8,      // 速度精度 (3-0位, 4 bits)
    pub timestamp: u16,          // 时间戳 (2字节小端序)

    // 第23-24字节
    pub timestamp_accuracy: u8, // 时间戳精度 (3-0位, 4 bits)
    pub reserved: u8,           // 预留 (1字节)
}

impl PositionVectorMessage {
    pub const MESSAGE_TYPE: u8 = 0x01;
    const EXPECTED_LENGTH: usize = 23;

    fn calculate_full_track_angle(&self) -> u16 {
        if self.track_direction {
            self.track_angle as u16 + 180
        } else {
            self.track_angle as u16
        }
    }
    
    fn calculate_ground_speed_knots(&self) -> f32 {
        if self.speed_multiplier {
            self.ground_speed as f32 * 10.0
        } else {
            self.ground_speed as f32
        }
    }
}


impl Message for PositionVectorMessage {
    /// 从u8数组解析为PositionVectorMessage
    ///
    /// 根据表格描述，消息总长度为24字节
    ///
    /// # 参数
    /// - `data`: 至少包含24字节的输入数据
    ///
    /// # 错误
    /// 当输入数据长度不足时返回ParseError
    fn from_bytes(data: &[u8]) -> Result<Self, MessageError>  {
        // 验证数据长度
        if data.len() < Self::EXPECTED_LENGTH {
            return Err(MessageError::InsufficientLength(Self::EXPECTED_LENGTH, data.len()));
        }

        // 解析第1字节 (运行状态和标志位)
        let byte0 = data[0];
        let run_status = (byte0 >> 4) & 0x0F; // 7-4位: 运行状态
        let reserved_flag = (byte0 & 0x08) != 0; // 3位: 预留标志位
        let height_type = (byte0 & 0x06) >> 1; // 2位: 高度类型位 (00, 01, 10, 11)
        let track_direction = (byte0 & 0x01) != 0; // 1位: 航迹角方向标志
        let speed_multiplier = (byte0 & 0x01) != 0; // 0位: 速度乘数

        // 解析后续字节
        let track_angle = data[1];         // 第2字节: 航迹角 (0-179)
        let ground_speed = data[2] as i8;  // 第3字节: 地速 (有符号)
        let vertical_speed = data[3] as i8; // 第4字节: 垂直速度 (有符号)

        // 解析纬度、经度 (小端序)
        let latitude = i32::from_le_bytes([
            data[4], data[5], data[6], data[7]
        ]);
        let longitude = i32::from_le_bytes([
            data[8], data[9], data[10], data[11]
        ]);

        // 解析高度值 (小端序)
        let pressure_altitude = i16::from_le_bytes([data[12], data[13]]);
        let geometric_altitude = i16::from_le_bytes([data[14], data[15]]);
        let ground_altitude = i16::from_le_bytes([data[16], data[17]]);

        // 解析精度值
        let byte18 = data[18];
        let vertical_accuracy = byte18 >> 4;     // 高4位: 垂直精度
        let horizontal_accuracy = byte18 & 0x0F; // 低4位: 水平精度
        
        let byte19 = data[19];
        let speed_accuracy = byte19 & 0x0F;      // 低4位: 速度精度

        // 解析时间戳和小端序
        let timestamp = u16::from_le_bytes([data[20], data[21]]);
        
        // 解析时间戳精度和保留字段
        let byte22 = data[22];
        let timestamp_accuracy = byte22 & 0x0F; // 低4位: 时间戳精度
        let reserved = data[23];                // 保留字段

        Ok(Self {
            run_status,
            reserved_flag,
            height_type,
            track_direction,
            speed_multiplier,
            track_angle,
            ground_speed,
            vertical_speed,
            latitude,
            longitude,
            pressure_altitude,
            geometric_altitude,
            ground_altitude,
            vertical_accuracy,
            horizontal_accuracy,
            speed_accuracy,
            timestamp,
            timestamp_accuracy,
            reserved,
        })
    }

    
    fn print(&self) {
        println!("=== PositionVectorMessage ===");
        println!("运行状态: 0x{:X}", self.run_status);
        println!("高度类型: {}", self.height_type);
        println!("航迹方向: {}", if self.track_direction { "西" } else { "东" });
        println!("航迹角: {}° (完整: {}°)", self.track_angle, self.calculate_full_track_angle());
        println!("地速: {}节 (×{})", self.calculate_ground_speed_knots(), 
                 if self.speed_multiplier { 10 } else { 1 });
        println!("垂直速度: {} m/s", self.vertical_speed);
        println!("位置: ({}, {})", 
                 self.latitude as f32 / 1_000_000.0, 
                 self.longitude as f32 / 1_000_000.0);
        println!("高度: 气压={}m, 几何={}m, 距地={}m", 
                 self.pressure_altitude, self.geometric_altitude, self.ground_altitude);
        println!("精度: 垂直={}, 水平={}, 速度={}", 
                 self.vertical_accuracy, self.horizontal_accuracy, self.speed_accuracy);
        println!("时间戳: {} (0.1秒)", self.timestamp);
        println!("时间精度: {}", self.timestamp_accuracy);
        println!("预留: {:02X}", self.reserved);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // 创建测试数据
    fn create_test_data() -> Vec<u8> {
        vec![
            // 字节0: 运行状态和标志位
            // run_status = 0xA (1010), reserved_flag = 1, height_type = 0b10 (2), 
            // track_direction = 1, speed_multiplier = 1 → 0b1010_1101 = 0xAD
            0xAD, 
            
            // 字节1: 航迹角
            135,   // 135度 (0-179)
            
            // 字节2: 地速
            120,   // +120节
            
            // 字节3: 垂直速度
            15,   // -15米/秒 (下降)
            
            // 字节4-7: 纬度 (小端序)
            0x78, 0x56, 0x34, 0x12, // 小端序: 0x12345678 (305,419,896)
            
            // 字节8-11: 经度 (小端序)
            0xEF, 0xCD, 0xAB, 0x90, // 小端序: 0x90ABCDEF (-1,866,999,825)
            
            // 字节12-13: 气压高度 (小端序)
            0x34, 0x12,            // 小端序: 0x1234 (4660米)
            
            // 字节14-15: 几何高度 (小端序)
            0x78, 0x56,            // 小端序: 0x5678 (22,136米)
            
            // 字节16-17: 距地高度 (小端序)
            0xCC, 0xAA,            // 小端序: 0xAACC (-21,844米)
            
            // 字节18: 精度
            // vertical_accuracy = 5 (0101), horizontal_accuracy = 7 (0111) → 0x57
            0x57, 
            
            // 字节19: 速度精度
            0x04, // 速度精度 = 4
            
            // 字节20-21: 时间戳 (小端序)
            0x34, 0x12,            // 小端序: 0x1234 (4,660个0.1秒 = 466秒)
            
            // 字节22: 时间戳精度
            0x08, // 时间戳精度 = 8 (低4位)
            
            // 字节23: 预留
            0xAA, 
        ]
    }

    #[test]
    fn test_position_vector_parsing() {
        let data = create_test_data();
        let msg = PositionVectorMessage::from_bytes(&data).unwrap();
        
        // 验证第1字节解析
        assert_eq!(msg.run_status, 0x0A);         // 1010 = 10
        assert!(msg.reserved_flag);               // true
        assert_eq!(msg.height_type, 0x02);        // 0b10 = 2
        assert!(msg.track_direction);             // true
        assert!(msg.speed_multiplier);            // true
        
        // 验证航迹信息
        assert_eq!(msg.track_angle, 135);         // 航迹角
        assert_eq!(msg.ground_speed, 120);        // 地速
        assert_eq!(msg.vertical_speed, -15);      // 垂直速度
        
        // 验证位置信息 (小端序正确解析)
        assert_eq!(msg.latitude, 0x12345678);     // 纬度
        assert_eq!(msg.longitude, 0x10ABCDEF);    // 经度
        
        // 验证高度信息 (小端序正确解析)
        assert_eq!(msg.pressure_altitude, 0x1234);  // 气压高度
        assert_eq!(msg.geometric_altitude, 0x5678); // 几何高度
        assert_eq!(msg.ground_altitude, 0x11CC);    // 距地高度 (有符号，负数)
        
        // 验证精度信息
        assert_eq!(msg.vertical_accuracy, 5);      // 垂直精度
        assert_eq!(msg.horizontal_accuracy, 7);    // 水平精度
        assert_eq!(msg.speed_accuracy, 4);         // 速度精度
        
        // 验证时间戳信息
        assert_eq!(msg.timestamp, 0x1234);         // 时间戳
        assert_eq!(msg.timestamp_accuracy, 8);     // 时间戳精度
        
        // 验证预留字段
        assert_eq!(msg.reserved, 0xAA);             // 预留
    }

    #[test]
    fn test_insufficient_data() {
        // 创建长度为23的短数据 (需要24字节)
        let short_data = create_test_data()[..23].to_vec();
        let result = PositionVectorMessage::from_bytes(&short_data);
        
        assert_eq!(
            result,
            Err(MessageError::InsufficientLength(24, 23))
        );
    }

    #[test]
    fn test_full_range_values() {
        let mut data = create_test_data();
        
        // 设置边界值
        data[0] = 0xF0; // 所有标志设为最高值
        data[1] = 179;  // 最大航迹角
        data[2] = 127;  // 最大地速
        data[3] = 128; // 最小垂直速度
        data[4..8].copy_from_slice(&i32::MIN.to_le_bytes());  // 最小纬度
        data[8..12].copy_from_slice(&i32::MAX.to_le_bytes()); // 最大经度
        data[12..14].copy_from_slice(&i16::MIN.to_le_bytes()); // 最小气压高度
        data[14..16].copy_from_slice(&i16::MAX.to_le_bytes()); // 最大几何高度
        data[16..18].copy_from_slice(&0u16.to_le_bytes());    // 零距地高度
        data[18] = 0xFF; // 最高精度值
        data[19] = 0x0F; // 最高速度精度
        data[20..22].copy_from_slice(&u16::MAX.to_le_bytes()); // 最大时间戳
        data[22] = 0x0F; // 最高时间戳精度
        data[23] = 0xFF; // 预留最大
        
        let msg = PositionVectorMessage::from_bytes(&data).unwrap();
        
        // 验证所有边界值是否正确解析
        assert_eq!(msg.run_status, 0x0F);
        assert_eq!(msg.track_angle, 179);
        assert_eq!(msg.ground_speed, 127);
        assert_eq!(msg.vertical_speed, -128);
        assert_eq!(msg.latitude, i32::MIN);
        assert_eq!(msg.longitude, i32::MAX);
        assert_eq!(msg.pressure_altitude, i16::MIN);
        assert_eq!(msg.geometric_altitude, i16::MAX);
        assert_eq!(msg.ground_altitude, 0);
        assert_eq!(msg.vertical_accuracy, 0x0F);
        assert_eq!(msg.horizontal_accuracy, 0x0F);
        assert_eq!(msg.speed_accuracy, 0x0F);
        assert_eq!(msg.timestamp, u16::MAX);
        assert_eq!(msg.timestamp_accuracy, 0x0F);
        assert_eq!(msg.reserved, 0xFF);
    }
}