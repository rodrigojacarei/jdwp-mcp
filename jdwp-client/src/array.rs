// ArrayReference command implementations
//
// Commands for working with arrays

use crate::commands::{command_sets, array_reference_commands};
use crate::connection::JdwpConnection;
use crate::protocol::{CommandPacket, JdwpResult};
use crate::reader::{read_i32, read_u8};
use crate::types::{ArrayId, Value, ValueData};
use bytes::{Buf, BufMut};

impl JdwpConnection {
    /// Get the length of an array (ArrayReference.Length command)
    pub async fn get_array_length(&mut self, array_id: ArrayId) -> JdwpResult<i32> {
        let id = self.next_id();
        let mut packet = CommandPacket::new(id, command_sets::ARRAY_REFERENCE, array_reference_commands::LENGTH);

        packet.data.put_u64(array_id);

        let reply = self.send_command(packet).await?;
        reply.check_error()?;

        let mut data = reply.data();
        let length = read_i32(&mut data)?;

        Ok(length)
    }

    /// Get values from an array (ArrayReference.GetValues command)
    pub async fn get_array_values(
        &mut self,
        array_id: ArrayId,
        first_index: i32,
        length: i32,
    ) -> JdwpResult<Vec<Value>> {
        let id = self.next_id();
        let mut packet = CommandPacket::new(id, command_sets::ARRAY_REFERENCE, array_reference_commands::GET_VALUES);

        packet.data.put_u64(array_id);
        packet.data.put_i32(first_index);
        packet.data.put_i32(length);

        let reply = self.send_command(packet).await?;
        reply.check_error()?;

        let mut data = reply.data();

        // Read array tag (element type)
        let array_tag = read_u8(&mut data)?;

        // Read number of values
        let values_count = read_i32(&mut data)?;
        let mut values = Vec::with_capacity(values_count as usize);

        // For primitive arrays, values don't have individual tags
        // For object arrays (tag 91 or 115+), each value has a tag
        let is_untagged = matches!(array_tag, 66 | 67 | 68 | 70 | 73 | 74 | 83 | 90);

        for _ in 0..values_count {
            let (tag, value_data) = if is_untagged {
                // Primitive array - use array_tag for all elements
                let value_data = read_value_by_tag(array_tag, &mut data)?;
                (array_tag, value_data)
            } else {
                // Object array - each element has its own tag
                let tag = read_u8(&mut data)?;
                let value_data = read_value_by_tag(tag, &mut data)?;
                (tag, value_data)
            };

            values.push(Value {
                tag,
                data: value_data,
            });
        }

        Ok(values)
    }
}

/// Read a value based on its type tag
fn read_value_by_tag(tag: u8, buf: &mut &[u8]) -> JdwpResult<ValueData> {
    match tag {
        66 => Ok(ValueData::Byte(buf.get_i8())),
        67 => Ok(ValueData::Char(buf.get_u16())),
        68 => Ok(ValueData::Double(buf.get_f64())),
        70 => Ok(ValueData::Float(buf.get_f32())),
        73 => Ok(ValueData::Int(buf.get_i32())),
        74 => Ok(ValueData::Long(buf.get_i64())),
        83 => Ok(ValueData::Short(buf.get_i16())),
        90 => Ok(ValueData::Boolean(buf.get_u8() != 0)),
        86 => Ok(ValueData::Void),
        76 | 115 | 116 | 103 | 108 | 99 | 91 => {
            let object_id = crate::reader::read_u64(buf)?;
            Ok(ValueData::Object(object_id))
        }
        _ => Err(crate::protocol::JdwpError::Protocol(format!("Unknown value tag: {}", tag))),
    }
}
