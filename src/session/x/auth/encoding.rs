use std::{
    io::{self, Read, Write},
    vec,
};

fn read_len<R: Read>(reader: &mut R) -> io::Result<u16> {
    let mut buffer = [0u8; 2];
    reader.read_exact(&mut buffer)?;
    Ok(u16::from_be_bytes(buffer))
}

fn write_len<W: Write>(writer: &mut W, value: u16) -> io::Result<()> {
    writer.write_all(&value.to_be_bytes())
}

fn read_field<R: Read>(reader: &mut R) -> io::Result<Vec<u8>> {
    // TODO: is a shared buffer for length here worth it?
    // Is the compiler smart enough to optimize?

    let len = read_len(reader)?;
    let mut buf = vec![0u8; len as usize];

    reader.read_exact(&mut buf).map(|_| buf)
}

fn err_invalid_field(field: &str) -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        format!("Invalid field: {field}"),
    )
}

macro_rules! read_field_into {
    ($reader:ident, $field:literal) => {
        read_field($reader)?
            .try_into()
            .map_err(|_| err_invalid_field($field))
    };
}

fn write_field(writer: &mut impl Write, bytes: &[u8]) -> io::Result<()> {
    let prefix = bytes.len() as u16;

    write_len(writer, prefix)?;
    writer.write_all(bytes)?;

    Ok(())
}

#[derive(Debug)]
pub enum Family {
    Local,
    Wild,
    Other(u16),
    // Netname, 254
    // Krb5Principal, 253
    // LocalHost, 252
}

impl Family {
    fn encode(&self) -> u16 {
        match self {
            Self::Local => 256,
            Self::Wild => 65535, // TODO:
            Self::Other(x) => *x,
        }
    }

    fn decode(value: u16) -> Self {
        match value {
            256 => Self::Local,
            65535 => Self::Wild,
            x => Self::Other(x),
        }
    }
}

#[derive(Debug)]
pub struct Entry {
    pub family: Family,
    pub address: Vec<u8>,
    pub display_number: String,
    pub auth_name: String,
    pub auth_data: Vec<u8>,
}

impl Entry {
    pub fn read_from<R: Read>(reader: &mut R) -> io::Result<Option<Self>> {
        let family = Family::decode(match read_len(reader) {
            Ok(value) => value,
            Err(e) => {
                return match e.kind() {
                    io::ErrorKind::UnexpectedEof => Ok(None),
                    _ => Err(e),
                };
            }
        });

        Ok(Some(Self {
            family,
            address: read_field_into!(reader, "address")?,
            display_number: read_field_into!(reader, "display_number")?,
            auth_name: read_field_into!(reader, "auth_name")?,
            auth_data: read_field_into!(reader, "auth_data")?,
        }))
    }

    pub fn write_to<W: Write>(&self, writer: &mut W) -> io::Result<()> {
        write_len(writer, self.family.encode())?;
        write_field(writer, &self.address)?;
        write_field(writer, self.display_number.as_bytes())?;
        write_field(writer, self.auth_name.as_bytes())?;
        write_field(writer, &self.auth_data)?;

        Ok(())
    }
}

pub type Hostname = Vec<u8>;

pub enum Target {
    // u16 (65536 cookies) is an arbitrary but reasonable limit
    Server { slot: u16 },
    Client { display_number: String },
}

impl From<Target> for String {
    fn from(value: Target) -> Self {
        match value {
            Target::Server { slot } => slot.to_string(),
            Target::Client { display_number } => display_number,
        }
    }
}

pub enum Scope {
    Local(Hostname),
    Any,
}

impl From<Scope> for (Family, Hostname) {
    fn from(value: Scope) -> Self {
        match value {
            Scope::Local(hostname) => (Family::Local, hostname),
            Scope::Any => (Family::Wild, [127, 0, 0, 2].to_vec()), // TODO: address
        }
    }
}

// Technically, this should be a trait "AuthMethod"
// Practically, cookie is the only method that is currently used
// TODO: do we need special memory handling here for security? zeroize on drop?
pub struct Cookie([u8; Self::BYTES_LEN]);
impl Cookie {
    pub const BYTES_LEN: usize = 16; // 16 * 8 = 128 random bits
    const AUTH_NAME: &str = "MIT-MAGIC-COOKIE-1";

    pub fn new(random_bytes: [u8; Self::BYTES_LEN]) -> Self {
        Self(random_bytes)
    }

    pub fn raw_data(&self) -> (String, Vec<u8>) {
        // TODO: return &str for name?
        (Self::AUTH_NAME.to_string(), self.0.into())
    }
}

impl Entry {
    pub fn new(cookie: &Cookie, scope: Scope, target: Target) -> Entry {
        let (family, address) = scope.into();
        let display_number = target.into();
        let (auth_name, auth_data) = cookie.raw_data();

        Entry {
            family,
            address,
            display_number,
            auth_name,
            auth_data,
        }
    }
}
