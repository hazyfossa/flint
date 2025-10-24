#![allow(dead_code)]
mod proto;

use anyhow::{Context, Result};
use bytes::BytesMut;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::UnixStream,
};

use proto::{Response, response};

#[macro_export]
macro_rules! rpc_method {
    // No argument
    ($method_name:ident, $char:expr, None, $response_type:ty) => {
        pub async fn $method_name(&mut self) -> Result<$response_type> {
            self.call(proto::Request {
                method: $char,
                argument: None,
            })
            .await
        }
    };

    // With argument
    ($method_name:ident, $char:expr, $arg_name:ident, $response_type:ty) => {
        pub async fn $method_name(&mut self, $arg_name: String) -> Result<$response_type> {
            self.call(proto::Request {
                method: $char,
                argument: Some($arg_name),
            })
            .await
        }
    };
}

pub struct Client {
    stream: UnixStream,
}

#[rustfmt::skip::macros(rpc_method)]
impl Client {
    pub async fn connnect() -> Result<Self> {
        // TODO: What's the actual (connection) path here?
        const SOCKET_PATH: &str = "\0/org/freedesktop/plymouthd";
        const OLD_SOCKET_PATH: &str = "\0/ply-boot-protocol";

        let stream = UnixStream::connect(SOCKET_PATH).await.unwrap_or(
            UnixStream::connect(OLD_SOCKET_PATH)
                .await
                .context("Failed to connect to plymouthd")?,
        );

        Ok(Self { stream })
    }

    pub async fn call<T: Response>(&mut self, request: proto::Request) -> Result<T> {
        self.stream.write_all(&request.serialize()?).await?;
        self.stream.flush().await?;

        let mut buf = BytesMut::new();
        self.stream.read_buf(&mut buf).await?; // TODO

        T::read_buf(&mut buf)
    }

    //                                                    TODO
    rpc_method!(ping,               'P',   None,          response::Simple);
    rpc_method!(update,             'U',   status,        response::Simple);
    rpc_method!(change_mode,        'C',   new_mode,      response::Simple);
    rpc_method!(system_update,      'u',   progress,      response::Simple);
    rpc_method!(system_initialized, 'S',   None,          response::Simple);
    rpc_method!(deactivate,         'D',   None,          response::Simple);
    rpc_method!(reactivate,         'r',   None,          response::Simple);
    rpc_method!(quit,               'Q',   retain_splash, response::Simple); // TODO: retain_splash is bool
    rpc_method!(reload,             'l',   None,          response::Simple);
    rpc_method!(ask_password,       '*',   prompt,        response::Simple);
    rpc_method!(cached_passwords,   'c',   None,          response::Simple);
    rpc_method!(ask_question,       'W',   prompt,        response::Simple);
    rpc_method!(show_message,       'M',   message,       response::Simple);
    rpc_method!(hide_message,       'm',   message,       response::Simple);
    rpc_method!(watch_keystroke,    'K',   keystroke,     response::Simple);
    rpc_method!(ignore_keystroke,   'L',   keystroke,     response::Simple);
    rpc_method!(progress_pause,     'A',   None,          response::Simple);
    rpc_method!(progress_unpause,   'a',   None,          response::Simple);
    rpc_method!(show_splash,        '$',   None,          response::Simple);
    rpc_method!(hide_splash,        'H',   None,          response::Simple);
    rpc_method!(newroot,            'R',   root_dir,      response::Simple);
    rpc_method!(has_active_vt,      'V',   None,          response::Simple);
    rpc_method!(error,              '!',   None,          response::Simple);
}
