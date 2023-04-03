use std::str;
use anyhow::{Result, Error};

const CRLF: &[u8] = b"\r\n";

#[derive(Eq, PartialEq, Clone, Debug)]
pub enum RESPMessage {
    SimpleString(String),
    Error(String),
    Integer(u64),
    BulkString(String),
    Array(Vec<RESPMessage>),
    Null
}

fn start_to_crlf(bytes: &[u8]) -> usize {
    let mut index = 0;
    for byte in bytes {
        if *byte == b'\r' {
            break;
        }
        index += 1;
    }
    index
}

impl RESPMessage {
    // Function that encodes data into the respective RESP format
    pub fn serialize(&self) -> Vec<u8> {
        let mut result: Vec<u8> = vec![];
        match self {
            Self::SimpleString(s) => {
                result.push(b'+');
                let mut string_bytes: Vec<u8> = s.as_bytes().to_owned();
                result.append(&mut string_bytes);
                result.append(&mut CRLF.to_owned());
            },
            Self::Error(s) => {
                result.push(b'-');
                let mut string_bytes: Vec<u8> = s.as_bytes().to_owned();
                result.append(&mut string_bytes);
                result.append(&mut CRLF.to_owned());
            },
            Self::Integer(i) => {
                result.push(b':');
                let mut integer_bytes: Vec<u8> = i.to_string().as_bytes().to_owned();
                result.append(&mut integer_bytes);
                result.append(&mut CRLF.to_owned());
            }
            Self::BulkString(s) => {
                result.push(b'$');

                let mut string_bytes: Vec<u8> = s.as_bytes().to_owned();
                let mut length_bytes: Vec<u8> = s.len().to_string().as_bytes().to_owned();

                result.append(&mut length_bytes);
                result.append(&mut CRLF.to_owned());
                result.append(&mut string_bytes);
                result.append(&mut CRLF.to_owned());
            },
            Self::Array(a) => {
                result.push(b'*');

                // Get the length of the array
                let mut length_bytes = a.len().to_string().as_bytes().to_owned();
                result.append(&mut length_bytes);
                result.append(&mut CRLF.to_owned());

                // Handle each element of the array
                for element in a {
                    let mut element_bytes = element.serialize();
                    result.append(&mut element_bytes);
                }
            },
            Self::Null => {
                result.push(b'$');
                // add b'-1' to the result
                let mut null_bytes = b"-1".to_vec();
                result.append(&mut null_bytes);
                result.append(&mut CRLF.to_owned());
            }
        }
        result
    }

    // Function that decodes data from the respective RESP format
    pub fn deserialize(bytes: &[u8]) -> (Self, usize) {
        match bytes[0] {
            b'+' => {
                let len: usize = start_to_crlf(&bytes[1..]);
                return (
                    Self::SimpleString(str::from_utf8(&bytes[1..len]).unwrap().to_string()),
                    len + 3, // +3 for the CRLF and the + 
                );
            },
            b'-' => {
                let len: usize = start_to_crlf(&bytes[1..]);
                return (
                    Self::Error(str::from_utf8(&bytes[1..len]).unwrap().to_string()),
                    len + 3, // +3 for the CRLF and the - 
                );
            },
            b':' => {
                let len: usize = start_to_crlf(&bytes[1..]);
                return (
                    Self::Integer(str::from_utf8(&bytes[1..len]).unwrap().parse().unwrap()),
                    len + 3, // +3 for the CRLF and the : 
                );
            },
            b'$' => {
                let len_len = start_to_crlf(&bytes[1..]);
                let len: usize = str::from_utf8(&bytes[1..=len_len])
                    .unwrap()
                    .parse()
                    .unwrap();

                return (
                    Self::BulkString(
                        str::from_utf8(&bytes[len_len + 3..len_len + 3 + len])
                            .unwrap()
                            .to_string(),
                    ),
                    len_len + 3 + len + 2,
                );
            },
            // TODO: potentially buggy
            b'*' => {
                let len_len = start_to_crlf(&bytes[1..]);
                let num_elements: usize = str::from_utf8(&bytes[1..=len_len])
                    .unwrap()
                    .parse()
                    .unwrap();

                let mut result: Vec<Self> = vec![];
                let mut used_length_in_elements = 0;
                let header_size = 1 + len_len + 2;

                for _ in 0..num_elements {
                    let (element, used_size) =
                        Self::deserialize(&bytes[header_size + used_length_in_elements..]);
                    result.push(element);
                    used_length_in_elements += used_size;
                }

                (Self::Array(result), header_size + used_length_in_elements)
            },
            _ => (Self::Error("Invalid RESP message type".to_string()), 0),
        }
    }

    // This is the function that we will use to convert the RESPMessage to a String
    pub fn pack_string(&self) -> Result<&str> {
        match self {
            Self::SimpleString(s) | Self::BulkString(s) => Ok(s),
            _ => Err(Error::msg("Trying to decode non-string")),
        }
    }

    pub fn to_command(&self) -> Result<(String, Vec<RESPMessage>), Error> {
        match self {
            RESPMessage::Array(elements) => {
                println!("Elements: {:?}", elements);
                if let Some(RESPMessage::BulkString(command)) = elements.get(0) {
                    let args: Vec<RESPMessage> = elements.iter().skip(1).cloned().collect();
                    Ok((command.clone(), args))
                } else {
                    Err(Error::msg("First element of the Array must be a BulkString"))
                }
            }
            _ => Err(Error::msg("Message is not an Array")),
        }
    }
    

    // pub fn to_command(&self) -> Result<(String, Vec<RESPMessage>)> {
    //     println!("MADE IT HERE");
    //     match self {
    //         RESPMessage::Array(elements) => {
    //             println!("MADE IT NOW");
    //             let (first, rest) = elements.split_first().unwrap();
    //             println!("MADE IT COOL");
    //             println!("First is here: {:?}", first);
    //             let first_str = match first {
    //                 RESPMessage::BulkString(s) => s.clone(),
    //                 _ => return Err(Error::msg("Invalid RESP message: not a BulkString.")),
    //             };
    //             let remaining: Vec<RESPMessage> = rest.iter().cloned().collect();
    //             println!("remaining is here: {:?}", remaining);
    //             Ok((first_str, remaining))
    //         },
    //         _ => Err(Error::msg("not an array")),
    //     }
    // }


}