use std::convert::TryInto;
use std::fmt;
use std::str;

use super::message::{Message, MessageError};

#[derive(Debug, Clone, PartialEq)]
pub struct BaseMessage {
    pub id_type: u8,          // 高位 4 位 (7-4 位)
    pub ua_type: u8,          // 低位 4 位 (3-0 位)
    pub uas_id: String,       // UAS 识别身份信息（字符串类型）
    pub reserved: [u8; 3],    // 3 字节预留空间
}

impl BaseMessage {
    pub const MESSAGE_TYPE: u8 = 0x00;
    const EXPECTED_LENGTH: usize = 23;
}

impl Message for BaseMessage {
    /// 从 u8 数组解析为结构化数据
    ///
    /// # 参数
    /// - `data`: 至少包含 24 字节的输入数据
    ///
    /// # 错误
    /// - 当输入数据长度不足时返回 ParseError::InsufficientLength
    /// - 当 UAS ID 不是有效的 UTF-8 时返回 ParseError::InvalidUtf8
    fn from_bytes(data: &[u8]) -> Result<Self, MessageError> {
        if data.len() < Self::EXPECTED_LENGTH{
            return Err(MessageError::InsufficientLength(
                Self::EXPECTED_LENGTH, 
                data.len()
            ));
        }

        // 解析第一个字节 (起始字节 1)
        let byte0 = data[0];
        let id_type = (byte0 >> 4) & 0x0F;  // 提取高4位 (7-4位)
        let ua_type = byte0 & 0x0F;         // 提取低4位 (3-0位)

        // 解析 UAS ID (起始字节 2，长度 20)
        let uas_id_start = 1;
        let uas_id_bytes = &data[uas_id_start..uas_id_start + 20];
        
        // 转换为 String，移除尾部的空字符(\0)和空白字符
        let uas_id = match str::from_utf8(uas_id_bytes) {
            Ok(s) => {
                // 移除尾部的空字符和空白字符
                s.trim_end_matches('\0')
                 .trim_end()
                 .to_string()
            }
            Err(e) => return Err(MessageError::InvalidUtf8(e)),
        };

        // 解析预留字段 (起始字节 22)
        let reserved_start = 21;  // 起始索引 = 起始字节 - 1
        let reserved: [u8; 3] = data[reserved_start..reserved_start + 3]
            .try_into()
            .map_err(|_| MessageError::InsufficientLength(24, data.len()))?;

        Ok(Self {
            id_type,
            ua_type,
            uas_id,
            reserved,
        })
    }


    fn print(&self) {
        println!("=== BaseMessage ===");
        println!("ID 类型: 0x{:X}", self.id_type);
        println!("UA 类型: 0x{:X}", self.ua_type);
        println!("UAS ID: '{}'", self.uas_id);
        println!("预留字段: {:02X?}", self.reserved);
    }
}



// 测试用例
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_parsing() {
        // 构造测试数据 (24字节)
        let mut data = vec![0u8; 24];
        // 设置第一个字节: 高4位=0xA (1010), 低4位=0x5 (0101)
        data[0] = 0xA5;
        
        // 设置 UAS ID 为带尾部空格的文本 (使用ASCII)
        let uas_id = b"DroneBase         "; // 19字符 + 1个空格
        data[1..21].copy_from_slice(uas_id);
        
        // 设置预留字段
        data[21] = 0xFE;
        data[22] = 0xED;
        data[23] = 0xCA;

        let msg = BaseMessage::from_bytes(&data).unwrap();
        
        // 验证字段解析
        assert_eq!(msg.id_type, 0xA);
        assert_eq!(msg.ua_type, 0x5);
        assert_eq!(msg.uas_id, "DroneBase"); // 尾部空格已移除
        assert_eq!(msg.reserved, [0xFE, 0xED, 0xCA]);
    }

    #[test]
    fn test_insufficient_data() {
        let short_data = vec![0u8; 23];  // 仅23字节
        let result = BaseMessage::from_bytes(&short_data);
        assert_eq!(result, Err(MessageError::InsufficientLength(24, 23)));
    }

    #[test]
    fn test_invalid_utf8() {
        let mut data = vec![0u8; 24];
        // 设置第一个字节
        data[0] = 0x34;
        // 在 UAS ID 部分插入无效的 UTF-8 序列
        data[1] = 0xFF; // 无效的 UTF-8 起始字节
        
        let result = BaseMessage::from_bytes(&data);
        assert!(matches!(result, Err(MessageError::InvalidUtf8(_))));
    }
    
    #[test]
    fn test_null_padding() {
        let mut data = vec![0u8; 24];
        data[0] = 0x12;
        
        // 带空终止符的有效 UAS ID
        let id = "BaseMsg\0\0\0\0\0\0\0\0\0\0\0\0\0";
        let id_bytes = id.as_bytes();
        data[1..21].copy_from_slice(&id_bytes[..id_bytes.len().min(20)]);
        
        // 如果长度不足20字节，补0
        if id_bytes.len() < 20 {
            data[1 + id_bytes.len()..21].fill(0);
        }
        
        let msg = BaseMessage::from_bytes(&data).unwrap();
        assert_eq!(msg.uas_id, "BaseMsg"); // 尾部空字符已移除
    }
}