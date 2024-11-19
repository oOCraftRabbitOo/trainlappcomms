#![warn(clippy::panic_in_result_fn)]
#![warn(clippy::missing_panics_doc)]

use super::*;
use bincode;
use futures::{SinkExt, StreamExt};
use std::io::{Error, ErrorKind};
use tokio::net::{
    tcp::{OwnedReadHalf, OwnedWriteHalf},
    TcpStream,
};
use tokio_util::codec::{FramedRead, FramedWrite, LengthDelimitedCodec};

pub struct TrainlappcommsSender {
    sender: FramedWrite<OwnedWriteHalf, LengthDelimitedCodec>,
}

impl TrainlappcommsSender {
    pub async fn send(&mut self, message: &ToServer) -> Result<(), Error> {
        match self
            .sender
            .send(
                bincode::serialize(message)
                    .map_err(|e| Error::new(ErrorKind::InvalidInput, e))?
                    .into(),
            )
            .await
        {
            Ok(_) => Ok(()),
            Err(e) => Err(Error::new(ErrorKind::Other, e)),
        }
    }
}

pub struct TrainlappcommsReceiver {
    receiver: FramedRead<OwnedReadHalf, LengthDelimitedCodec>,
}

impl TrainlappcommsReceiver {
    pub async fn recv(&mut self) -> Result<ToApp, Error> {
        Ok(
            bincode::deserialize(&self.receiver.next().await.ok_or(Error::new(
                ErrorKind::ConnectionAborted,
                "connection with trainlappcomms ended",
            ))??)
            .map_err(|e| {
                Error::new(
                    ErrorKind::InvalidData,
                    format!("message from trainappcomms couldn't be decoded: {}", e),
                )
            })?,
        )
    }
}

pub async fn connect() -> Result<(TrainlappcommsReceiver, TrainlappcommsSender), Error> {
    let (rx, tx) = TcpStream::connect("trainlag.ch:41314").await?.into_split();
    Ok((
        TrainlappcommsReceiver {
            receiver: FramedRead::new(rx, LengthDelimitedCodec::new()),
        },
        TrainlappcommsSender {
            sender: FramedWrite::new(tx, LengthDelimitedCodec::new()),
        },
    ))
}
