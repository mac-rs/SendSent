use std::io;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use uuid::Uuid;
use crate::proto::messages::*;

pub struct DataChunk { pub file_id: Uuid, pub offset: u64, pub data: Vec<u8> }

pub async fn write_control<W: AsyncWriteExt + Unpin>(w: &mut W, ty: MsgType, payload: &[u8]) -> io::Result<()> {
    if payload.len() as u32 > MAX_CONTROL_PAYLOAD {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "control payload too large"));
    }
    let mut hdr = [0u8; 7];
    hdr[0] = MAGIC; hdr[1] = PROTO_VER; hdr[2] = ty as u8;
    hdr[3..7].copy_from_slice(&(payload.len() as u32).to_be_bytes());
    w.write_all(&hdr).await?; w.write_all(payload).await?; Ok(())
}

pub async fn read_control<R: AsyncReadExt + Unpin>(r: &mut R) -> io::Result<(MsgType, Vec<u8>)> {
    let mut hdr = [0u8; 7];
    r.read_exact(&mut hdr).await?;
    if hdr[0] != MAGIC { return Err(io::Error::new(io::ErrorKind::InvalidData, "bad magic")); }
    if hdr[1] != PROTO_VER { return Err(io::Error::new(io::ErrorKind::InvalidData, "incompatible version")); }
    let ty = MsgType::try_from(hdr[2]).map_err(|()| io::Error::new(io::ErrorKind::InvalidData, "bad msg type"))?;
    let len = u32::from_be_bytes(hdr[3..7].try_into().unwrap());
    if len > MAX_CONTROL_PAYLOAD { return Err(io::Error::new(io::ErrorKind::InvalidData, "control payload too large")); }
    let mut buf = vec![0u8; len as usize];
    r.read_exact(&mut buf).await?; Ok((ty, buf))
}

pub async fn write_data<W: AsyncWriteExt + Unpin>(w: &mut W, file_id: Uuid, offset: u64, data: &[u8]) -> io::Result<()> {
    if data.len() as u32 > MAX_DATA_PAYLOAD {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "data payload too large"));
    }
    let mut hdr = [0u8; 29];
    hdr[0] = DATA_TAG;
    hdr[1..17].copy_from_slice(file_id.as_bytes());
    hdr[17..25].copy_from_slice(&offset.to_be_bytes());
    hdr[25..29].copy_from_slice(&(data.len() as u32).to_be_bytes());
    w.write_all(&hdr).await?; w.write_all(data).await?; Ok(())
}

pub async fn read_data<R: AsyncReadExt + Unpin>(r: &mut R) -> io::Result<DataChunk> {
    let mut hdr = [0u8; 29];
    r.read_exact(&mut hdr).await?;
    if hdr[0] != DATA_TAG { return Err(io::Error::new(io::ErrorKind::InvalidData, "bad data tag")); }
    let idb: [u8; 16] = hdr[1..17].try_into().unwrap();
    let offset = u64::from_be_bytes(hdr[17..25].try_into().unwrap());
    let len = u32::from_be_bytes(hdr[25..29].try_into().unwrap());
    if len > MAX_DATA_PAYLOAD { return Err(io::Error::new(io::ErrorKind::InvalidData, "data payload too large")); }
    let mut data = vec![0u8; len as usize];
    r.read_exact(&mut data).await?;
    Ok(DataChunk { file_id: Uuid::from_bytes(idb), offset, data })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn control_roundtrip() {
        let mut buf = Vec::<u8>::new();
        write_control(&mut buf, MsgType::Manifest, b"hello").await.unwrap();
        let (ty, payload) = read_control(&mut &buf[..]).await.unwrap();
        assert_eq!(ty, MsgType::Manifest); assert_eq!(payload, b"hello");
    }
    #[tokio::test]
    async fn data_roundtrip() {
        let id = Uuid::new_v4();
        let mut buf = Vec::<u8>::new();
        write_data(&mut buf, id, 4096, b"abcd").await.unwrap();
        let c = read_data(&mut &buf[..]).await.unwrap();
        assert_eq!(c.file_id, id); assert_eq!(c.offset, 4096); assert_eq!(c.data, b"abcd");
    }
    #[tokio::test]
    async fn bad_magic_rejected() {
        let buf = [0x00u8, PROTO_VER, 0x01, 0, 0, 0, 0];
        let err = read_control(&mut &buf[..]).await.unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidData);
    }
    #[tokio::test]
    async fn oversize_payload_rejected() {
        let mut buf = Vec::<u8>::new();
        let big = vec![0u8; (MAX_CONTROL_PAYLOAD + 1) as usize];
        let err = write_control(&mut buf, MsgType::Manifest, &big).await.unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
    }
}
