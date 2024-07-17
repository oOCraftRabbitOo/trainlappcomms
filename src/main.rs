use bincode;
use futures::prelude::*;
use tokio;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{broadcast, mpsc};
use tokio_util::codec::{FramedRead, FramedWrite, LengthDelimitedCodec};
use trainlappcomms::*;
use truinlag::api;
use truinlag::commands::{BroadcastAction, EngineAction, EngineCommand, ResponseAction};
use truinlag::TeamRole;

fn response_to_to_app(response: ResponseAction, player_id: u64) -> Option<ToApp> {
    use ResponseAction::*;
    match response {
        Error(_) => None,
        Team(_) => None,
        Player(_) => None,
        Success => None,
        SendState { teams, game } => {
            let your_team = teams
                .iter()
                .position(|t| t.players.iter().find(|p| p.id == player_id).is_some())
                .unwrap();
            let state = match game {
                None => State::GameNotRunning,
                Some(_) => match teams[your_team].role {
                    TeamRole::Catcher => State::Catcher,
                    TeamRole::Runner => State::Runner,
                },
            };
            Some(ToApp::Everything(Everything {
                state,
                teams: teams.into_iter().map(|t| t.into()).collect(),
                you: player_id,
                your_team,
            }))
        }
    }
}

fn broadcast_to_to_app(broadcast: BroadcastAction) -> ToApp {
    use BroadcastAction::*;
    match broadcast {
        Catch {
            catcher,
            caught,
        } => ,
    }
}

fn to_server_to_engine_command(to_server: trainlappcomms::ToServer) -> EngineCommand {
    todo!();
}

async fn handle_client(stream: TcpStream) -> Result<(), api::error::Error> {
    let (tcp_rx, tcp_tx) = stream.into_split();
    let mut transport_rx = FramedRead::new(tcp_rx, LengthDelimitedCodec::new());
    let mut transport_tx = FramedWrite::new(tcp_tx, LengthDelimitedCodec::new());

    let (mut truin_tx, truin_rx) = api::connect(None).await?;
    let (internal_tx, mut internal_rx) = mpsc::unbounded_channel();
    let internal_tx_2 = internal_tx.clone();

    let player_id = loop {
        if let ToServer::Login(passphrase) = bincode::deserialize::<trainlappcomms::ToServer>(
            &transport_rx.next().await.unwrap().unwrap(),
        )
        .unwrap()
        {
            match truin_tx
                .send(EngineCommand {
                    session: None,
                    action: EngineAction::GetPlayerByPassphrase(passphrase),
                })
                .await
                .unwrap()
            {
                ResponseAction::Player(id) => break id,
                _ => {
                    transport_tx.send(
                        bincode::serialize(&trainlappcomms::ToApp::LoginSuccessful(false))
                            .unwrap()
                            .into(),
                    );
                }
            }
        }
    };

    let app_receiver = async move {
        while let Some(message) = transport_rx.next().await {
            let message = message.unwrap();
            let message = bincode::deserialize::<trainlappcomms::ToServer>(&message).unwrap();
            let message = to_server_to_engine_command(message);
            match truin_tx.send(message).await {
                Ok(response) => {
                    if let Some(response) = response_to_to_app(response, player_id) {
                        internal_tx_2.send(response).unwrap()
                    }
                }
                Err(_) => break,
            }
        }
    };

    let app_sender = async move {
        loop {
            let message = internal_rx.recv().await.unwrap();
            transport_tx.send(bincode::serialize(&message).unwrap().into());
        }
    };

    let truin_receiver = async move {
        let mut truin_rx = truin_rx.activate().await;
        loop {
            if let Some(message) = truin_rx.recv().await {
                internal_tx.send(broadcast_to_to_app(message)).unwrap()
            }
        }
    };

    tokio::select! {
        _ = app_sender => (),
        _ = app_receiver => (),
        _ = truin_receiver => (),
    }
    todo!();
    let mut buffer = [0; 512];
    //loop {
    //    match stream.read(&mut buffer) {
    //        Ok(0) => {
    //            println!("A client disconnected... :(");
    //            break;
    //        } // connection was closed
    //        Ok(n) => {
    //            // Echo the received data back to the client
    //            let msg = String::from_utf8(Vec::from(&buffer[..n])).unwrap();
    //            println!("Received {}", msg);
    //            tx.send(msg).unwrap();
    //        }
    //        Err(e) => {
    //            eprintln!("Failed to read from stream: {}", e);
    //            break;
    //        }
    //    }
    //}
}

#[tokio::main()]
async fn main() -> std::io::Result<()> {
    let listener = TcpListener::bind("192.168.50.69:7878").await?;
    println!("Server listening on port 7878");

    loop {
        let accepted = listener.accept().await;
        match accepted {
            Ok((stream, addr)) => {
                println!("A client connected from {}", addr);
                tokio::spawn(handle_client(stream));
            }
            Err(e) => {
                eprintln!("Connection failed: {}", e);
            }
        }
    }
}
