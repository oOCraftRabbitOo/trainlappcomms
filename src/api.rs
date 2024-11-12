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
    pub async fn send(&mut self, message: &ToServer) -> Result<(), ()> {
        match self
            .sender
            .send(bincode::serialize(message).map_err(|_| ())?.into())
            .await
        {
            Ok(_) => Ok(()),
            Err(_) => Err(()),
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

pub async fn connect() -> (TrainlappcommsReceiver, TrainlappcommsSender) {
    let (rx, tx) = TcpStream::connect("trainlag.ch:41314")
        .await
        .expect("couldn't establish connection to server :(")
        .into_split();
    (
        TrainlappcommsReceiver {
            receiver: FramedRead::new(rx, LengthDelimitedCodec::new()),
        },
        TrainlappcommsSender {
            sender: FramedWrite::new(tx, LengthDelimitedCodec::new()),
        },
    )
}
