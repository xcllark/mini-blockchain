use std::io::Cursor;

use alloy_primitives::B256;
use serde::{Deserialize, Serialize};

use crate::{Error, SealedBlock, Transaction};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum Message {
    Transaction(Transaction),
    Block(SealedBlock),

    Blocks(Vec<SealedBlock>),

    BlockReq(BlockReq),
    TransactionReq(TransactionReq),

    NonExistentBlock,
    NonExistentTx,

    InvalidMessage(String),
    InvalidTransaction,

    InternalError(String),
    Ok,
}

impl Message {
    pub fn serialize(&self) -> Result<Vec<u8>, Error> {
        serde_json::to_vec(self).map_err(|e| e.into())
    }

    pub fn _deserialize(bytes: &[u8]) -> Result<Self, Error> {
        serde_json::from_slice(bytes).map_err(|e| e.into())
    }

    pub fn check(src: &mut Cursor<&[u8]>) -> Result<(), Error> {
        Self::get_line(src)?;
        Ok(())
    }

    pub fn parse(src: &mut Cursor<&[u8]>) -> Result<Message, Error> {
        let line = Self::get_line(src)?;
        let msg = serde_json::from_slice::<Message>(line)?;
        Ok(msg)
    }

    pub fn get_line<'a>(src: &mut Cursor<&'a [u8]>) -> Result<&'a [u8], Error> {
        // Scan the bytes directly
        let start = src.position() as usize;
        // Scan to the second to last byte
        if src.get_ref().len() < 2 {
            return Err(Error::IncompleteMessage);
        }
        let end = src.get_ref().len() - 1;

        for i in start..end {
            if src.get_ref()[i] == b'\r' && src.get_ref()[i + 1] == b'\n' {
                // We found a line, update the position to be *after* the \n
                src.set_position((i + 2) as u64);

                // Return the line
                return Ok(&src.get_ref()[start..i]);
            }
        }

        Err(Error::IncompleteMessage)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BlockReq {
    Range { start: u64, end: u64 },
    Number(u64),
    Hash(B256),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransactionReq {
    Many(Vec<B256>),
    Hash(B256),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_serialize_message() {
        let msg = Message::Transaction(Transaction::default());
        let bytes = serde_json::to_vec(&msg).unwrap();
        let de: Message = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(msg, de);

        let msg = Message::Block(SealedBlock::default());
        let bytes = serde_json::to_vec(&msg).unwrap();
        let de: Message = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(msg, de);

        let msg = Message::Blocks(Vec::new());
        let bytes = serde_json::to_vec(&msg).unwrap();
        let de: Message = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(msg, de);

        let msg = Message::BlockReq(BlockReq::Number(0));
        let bytes = serde_json::to_vec(&msg).unwrap();
        let de: Message = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(msg, de);

        let msg = Message::BlockReq(BlockReq::Hash(B256::ZERO));
        let bytes = serde_json::to_vec(&msg).unwrap();
        let de: Message = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(msg, de);

        let msg = Message::BlockReq(BlockReq::Range { start: 0, end: 10 });
        let bytes = serde_json::to_vec(&msg).unwrap();
        let de: Message = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(msg, de);

        let msg = Message::TransactionReq(TransactionReq::Hash(B256::ZERO));
        let bytes = serde_json::to_vec(&msg).unwrap();
        let de: Message = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(msg, de);

        let msg = Message::TransactionReq(TransactionReq::Many(Vec::new()));
        let bytes = serde_json::to_vec(&msg).unwrap();
        let de: Message = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(msg, de);

        let msg = Message::NonExistentBlock;
        let bytes = serde_json::to_vec(&msg).unwrap();
        let de: Message = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(msg, de);

        let msg = Message::NonExistentTx;
        let bytes = serde_json::to_vec(&msg).unwrap();
        let de: Message = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(msg, de);

        let msg = Message::InvalidMessage(String::new());
        let bytes = serde_json::to_vec(&msg).unwrap();
        let de: Message = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(msg, de);

        let msg = Message::InvalidTransaction;
        let bytes = serde_json::to_vec(&msg).unwrap();
        let de: Message = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(msg, de);

        let msg = Message::InternalError(String::new());
        let bytes = serde_json::to_vec(&msg).unwrap();
        let de: Message = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(msg, de);

        let msg = Message::Ok;
        let bytes = serde_json::to_vec(&msg).unwrap();
        let de: Message = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(msg, de);
    }
}
