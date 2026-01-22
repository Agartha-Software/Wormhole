use crate::network::message::{MessageAnswer, MessageContent};
use async_trait::async_trait;
use futures::prelude::*;
use libp2p::request_response::Codec;
use libp2p::StreamProtocol;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::io;

#[derive(Clone, Default)]
pub struct BincodeCodec {}

impl BincodeCodec {
    async fn read_message<T, M>(&mut self, io: &mut T) -> io::Result<M>
    where
        T: AsyncRead + Unpin + Send,
        M: DeserializeOwned,
    {
        let mut recived_answer = Vec::new();

        io.read_to_end(&mut recived_answer).await?;
        bincode::deserialize::<M>(recived_answer.as_slice()).map_err(std::io::Error::other)
    }

    async fn write_message<T, M>(&mut self, io: &mut T, message: M) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
        M: Serialize,
    {
        let serialized = bincode::serialize(&message).map_err(std::io::Error::other)?;
        io.write_all(&serialized).await?;
        Ok(())
    }
}

#[async_trait]
impl Codec for BincodeCodec {
    type Protocol = StreamProtocol;
    type Request = MessageContent;
    type Response = MessageAnswer;

    async fn read_request<T>(&mut self, _: &Self::Protocol, io: &mut T) -> io::Result<Self::Request>
    where
        T: AsyncRead + Unpin + Send,
    {
        self.read_message(io).await
    }

    async fn read_response<T>(
        &mut self,
        _: &Self::Protocol,
        io: &mut T,
    ) -> io::Result<Self::Response>
    where
        T: AsyncRead + Unpin + Send,
    {
        self.read_message(io).await
    }

    async fn write_request<T>(
        &mut self,
        _: &Self::Protocol,
        io: &mut T,
        req: Self::Request,
    ) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        self.write_message(io, req).await
    }

    async fn write_response<T>(
        &mut self,
        _: &Self::Protocol,
        io: &mut T,
        resp: Self::Response,
    ) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        self.write_message(io, resp).await
    }
}
