use serde::{de::DeserializeOwned, Serialize};
use std::io;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

use crate::model::MAX_FRAME_LEN;

#[derive(Debug, thiserror::Error)]
pub enum ProtocolError {
    #[error("frame too large: {0} bytes")]
    FrameTooLarge(usize),
    #[error("connection closed while reading frame")]
    Closed,
    #[error("codec error: {0}")]
    Codec(String),
    #[error("io error: {0}")]
    Io(#[from] io::Error),
}

pub fn encode<T: Serialize>(value: &T) -> Result<Vec<u8>, ProtocolError> {
    let mut buf = Vec::with_capacity(256);
    ciborium::into_writer(value, &mut buf).map_err(|e| ProtocolError::Codec(e.to_string()))?;
    Ok(buf)
}

pub fn decode<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, ProtocolError> {
    ciborium::from_reader(bytes).map_err(|e| ProtocolError::Codec(e.to_string()))
}

pub async fn send_frame<W>(writer: &mut W, payload: &[u8]) -> Result<(), ProtocolError>
where
    W: AsyncWrite + Unpin,
{
    if payload.len() > MAX_FRAME_LEN {
        return Err(ProtocolError::FrameTooLarge(payload.len()));
    }
    let len =
        u32::try_from(payload.len()).map_err(|_| ProtocolError::FrameTooLarge(payload.len()))?;
    writer.write_all(&len.to_be_bytes()).await?;
    writer.write_all(payload).await?;
    writer.flush().await?;
    Ok(())
}

pub async fn recv_frame<R>(reader: &mut R) -> Result<Vec<u8>, ProtocolError>
where
    R: AsyncRead + Unpin,
{
    let mut len_buf = [0u8; 4];
    match reader.read_exact(&mut len_buf).await {
        Ok(_) => {}
        Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => return Err(ProtocolError::Closed),
        Err(e) => return Err(ProtocolError::Io(e)),
    }
    let len = u32::from_be_bytes(len_buf) as usize;
    if len == 0 {
        return Err(ProtocolError::Codec("empty frame".into()));
    }
    if len > MAX_FRAME_LEN {
        return Err(ProtocolError::FrameTooLarge(len));
    }
    let mut payload = vec![0u8; len];
    reader.read_exact(&mut payload).await?;
    Ok(payload)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Request;

    #[tokio::test]
    async fn frame_roundtrip() {
        let payload = encode(&Request::Health).unwrap();
        let mut buf = Vec::new();
        send_frame(&mut buf, &payload).await.unwrap();
        assert_eq!(&buf[..4], &(payload.len() as u32).to_be_bytes());
        let mut cursor = std::io::Cursor::new(buf);
        let received = recv_frame(&mut cursor).await.unwrap();
        let req: Request = decode(&received).unwrap();
        assert_eq!(req, Request::Health);
    }

    #[tokio::test]
    async fn rejects_oversized_frame() {
        let mut reader = std::io::Cursor::new(vec![0xff, 0xff, 0xff, 0xff]);
        let err = recv_frame(&mut reader).await.unwrap_err();
        assert!(matches!(err, ProtocolError::FrameTooLarge(_)));
    }
}
