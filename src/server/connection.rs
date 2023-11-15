use super::Message;
use crate::Error;
use bytes::{Buf, BytesMut};
use std::io::Cursor;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt, BufWriter},
    net::TcpStream,
};

const BUFFER_SIZE: usize = 1024 * 4;

pub struct Connection {
    stream: BufWriter<TcpStream>,
    buffer: BytesMut,
}

impl Connection {
    pub fn new(stream: TcpStream) -> Self {
        Self {
            stream: BufWriter::new(stream),
            buffer: BytesMut::with_capacity(BUFFER_SIZE),
        }
    }

    pub async fn shutdown(self) {
        let _ = self.stream.into_inner().shutdown().await;
    }

    pub async fn read_message(&mut self) -> Result<Option<Message>, Error> {
        loop {
            if let Some(msg) = self.parse_message().await? {
                return Ok(Some(msg));
            }

            if 0 == self.stream.read_buf(&mut self.buffer).await? {
                if self.buffer.is_empty() {
                    return Ok(None);
                } else {
                    return Err(Error::ConnectionEnded);
                }
            }
        }
    }

    pub async fn parse_message(&mut self) -> Result<Option<Message>, Error> {
        let mut buf = Cursor::new(&self.buffer[..]);

        match Message::check(&mut buf) {
            Ok(_) => {
                let len = buf.position() as usize;

                buf.set_position(0);

                let message = Message::parse(&mut buf)?;

                self.buffer.advance(len);

                Ok(Some(message))
            }

            Err(Error::IncompleteMessage) => Ok(None),
            Err(e) => Err(e),
        }
    }

    pub async fn write_message(&mut self, message: &Message) -> Result<(), Error> {
        let serialized_message = message.serialize()?;
        self.stream.write_all(&serialized_message).await?;
        self.stream.write_all(b"\r\n").await?;
        self.stream.flush().await?;
        Ok(())
    }
}
