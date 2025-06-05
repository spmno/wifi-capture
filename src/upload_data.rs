use serde::Serialize;
#[derive(Serialize)]
pub struct UploadData {
    pub rid: String,
    pub run_status: u8,
    pub reserved_flag: bool,
    pub height_type: u8,
    pub track_direction: bool,
    pub speed_multiplier: bool,
    pub track_angle: u8,
    pub ground_speed: i8,
    pub vertical_speed: i8,
    pub latitude: i32,
    pub longitude: i32,
    pub pressure_altitude: i16,
    pub geometric_altitude: i16,
    pub ground_altitude: i16,
    pub vertical_accuracy: u8,
    pub horizontal_accuracy: u8,
    pub speed_accuracy: u8,
    pub timestamp: u16,
    pub timestamp_accuracy: u8,
    pub reserved: u8,
}