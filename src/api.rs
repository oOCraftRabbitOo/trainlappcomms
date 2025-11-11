#![warn(clippy::panic_in_result_fn)]

use super::*;
use bincode;
use futures::{SinkExt, StreamExt};
use std::io::{Error, ErrorKind};
use tokio::{
    io::AsyncWriteExt,
    net::{
        tcp::{OwnedReadHalf, OwnedWriteHalf},
        TcpStream,
    },
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
            Err(e) => Err(Error::other(e)),
        }
    }
}

pub struct TrainlappcommsReceiver {
    receiver: FramedRead<OwnedReadHalf, LengthDelimitedCodec>,
}

impl TrainlappcommsReceiver {
    pub async fn recv(&mut self) -> Result<ToApp, Error> {
        bincode::deserialize(&self.receiver.next().await.ok_or(Error::new(
            ErrorKind::ConnectionAborted,
            "connection with trainlappcomms ended",
        ))??)
        .map_err(|e| {
            Error::new(
                ErrorKind::InvalidData,
                format!("message from trainappcomms couldn't be decoded: {}", e),
            )
        })
    }
}

pub async fn connect() -> Result<(TrainlappcommsReceiver, TrainlappcommsSender), Error> {
    let (rx, tx) = TcpStream::connect(
        if cfg!(debug_assertions) || option_env!("TL_DEBUG").is_some() {
            "trainlag.ch:42314"
        } else {
            "trainlag.ch:41314"
        },
    )
    .await?
    .into_split();
    Ok((
        TrainlappcommsReceiver {
            receiver: FramedRead::new(rx, LengthDelimitedCodec::new()),
        },
        TrainlappcommsSender {
            sender: FramedWrite::new(tx, LengthDelimitedCodec::new()),
        },
    ))
}

pub async fn send_team_picture(
    picture: Vec<u8>,
    session: u64,
    team: usize,
) -> Result<(), std::io::Error> {
    let wrapper = PictureWrapper {
        kind: PictureKind::TeamProfile { session, team },
        picture,
    };
    send_picture(wrapper).await
}

pub async fn send_player_picture(picture: Vec<u8>, player: u64) -> Result<(), std::io::Error> {
    let wrapper = PictureWrapper {
        kind: PictureKind::PlayerProfile(player),
        picture,
    };
    send_picture(wrapper).await
}

pub async fn send_period_picture(
    picture: Vec<u8>,
    session: u64,
    team: usize,
    period_id: usize,
) -> Result<(), std::io::Error> {
    let wrapper = PictureWrapper {
        kind: PictureKind::Period {
            session,
            team,
            period_id,
        },
        picture,
    };
    send_picture(wrapper).await
}

async fn send_picture(pic: PictureWrapper) -> Result<(), std::io::Error> {
    let message = bincode::serialize(&pic).unwrap();
    let mut connection = TcpStream::connect(if cfg!(debug_assertions) {
        "trainlag.ch:42315"
    } else {
        "trainlag.ch:41315"
    })
    .await?;
    connection.write_all(&message).await?;
    connection.shutdown().await?;
    Ok(())
}
