use anyhow::{Context, Result, bail};
use binrw::binrw;
use shrinkwraprs::Shrinkwrap;

use std::ffi::{CString, c_char};

use crate::utils::bufio::{Buf, BufRead};

#[derive(Debug)]
pub struct Request {
    pub method: char,
    pub argument: Option<String>,
}

impl Request {
    pub fn serialize(self) -> Result<Vec<u8>> {
        let argument: String = self.argument.unwrap_or_default();

        Ok(CString::new(format!(
            "{}\x02{}{}",
            self.method,
            argument.len() as c_char,
            argument
        ))
        .context("Cannot serialize request as a C string")?
        .into())
    }
}

#[binrw]
#[brw(big)]
#[brw(repr = u8)]
#[derive(Debug)]
pub enum ResponseCode {
    Ack = 6,
    Nak = 15,
    Answer = 2,
    NoAnswer = 5,
    MultipleAnswers = 9,
}

pub trait Response: Sized {
    fn process(code: ResponseCode, buf: &mut impl Buf) -> Result<Self>;

    fn read_buf(buf: &mut impl Buf) -> Result<Self> {
        let code = ResponseCode::read_buf(buf)?;
        Self::process(code, buf)
    }
}

pub mod response {
    use super::*;

    #[derive(Shrinkwrap)]
    pub struct Simple(bool);
    impl Response for Simple {
        fn process(code: ResponseCode, _: &mut impl Buf) -> Result<Self> {
            Ok(Self(match code {
                ResponseCode::Ack => true,
                ResponseCode::Nak => false,
                other_code => bail!("Received unexpected response code: {other_code:?}"),
            }))
        }
    }

    #[derive(Shrinkwrap)]
    pub struct Answer(Option<String>);
    impl Response for Answer {
        fn process(code: ResponseCode, buf: &mut impl Buf) -> Result<Self> {
            Ok(Self(match code {
                ResponseCode::Answer => {
                    let len = buf.get_u32() as usize * size_of::<c_char>();
                    let data = buf.copy_to_bytes(len);

                    Some(String::from_utf8(data.to_vec()).context("Invalid response content")?)
                }
                ResponseCode::NoAnswer => None,
                other_code => bail!("Received unexpected response code: {other_code:?}"),
            }))
        }
    }

    // TODO: is single Answer valid here?
    #[derive(Shrinkwrap)]
    pub struct MultipleAnswers(Vec<String>);
    impl Response for MultipleAnswers {
        fn process(code: ResponseCode, buf: &mut impl Buf) -> Result<Self> {
            Ok(Self(match code {
                ResponseCode::MultipleAnswers => {
                    let len = buf.get_u32() as usize;
                    let data = buf.copy_to_bytes(len);

                    let mut strings = Vec::new();
                    for raw_string in data.split(|char| *char as char == '\0') {
                        strings.push(
                            String::from_utf8(raw_string.to_vec())
                                .context("Invalid response content")?,
                        );
                    }
                    strings
                }
                ResponseCode::NoAnswer => Vec::new(),
                other_code => bail!("Received unexpected response code: {other_code:?}"),
            }))
        }
    }
}
