use bincode;
use futures::prelude::*;
use tokio;
use tokio::net::tcp::OwnedWriteHalf;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;
use tokio_util::codec::{FramedRead, FramedWrite, LengthDelimitedCodec};
use trainlappcomms::*;
use truinlag::api;
use truinlag::commands::{BroadcastAction, EngineAction, EngineCommand, ResponseAction};
use truinlag::TeamRole;

async fn get_everything(player_id: u64, truin_tx: &mut api::SendConnection) -> Everything {
    match response_to_to_app(
        truin_tx
            .send(EngineCommand {
                session: None,
                action: EngineAction::GetState,
            })
            .await
            .unwrap(),
        player_id,
    )
    .unwrap()
    {
        ToApp::Everything(everything) => everything,
        _ => panic!(),
    }
}

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

async fn broadcast_to_to_app(
    broadcast: BroadcastAction,
    player_id: u64,
    truin_tx: &mut api::SendConnection,
) -> ToApp {
    use BroadcastAction::*;
    match broadcast {
        Location { team, location } => ToApp::Location { team, location },
        Caught { catcher, caught } => {
            let everything = get_everything(player_id, truin_tx).await;
            if catcher.players.iter().any(|p| p.id == player_id) {
                return ToApp::BecomeRunner(everything);
            } else if caught.players.iter().any(|p| p.id == player_id) {
                return ToApp::BecomeCatcher(everything);
            };
            ToApp::Everything(everything)
        }
        Completed {
            completer: _,
            completed: _,
        } => ToApp::Everything(get_everything(player_id, truin_tx).await),
        Pinged(mayssage) => ToApp::Ping(mayssage),
        Ended => ToApp::BecomeShutDown,
        Started => todo!(),
    }
}

fn to_server_to_engine_command(
    to_server: ToServer,
    session: u64,
    team_id: usize,
    player_id: u64,
) -> EngineCommand {
    use ToServer::*;
    match to_server {
        Login(passphrase) => EngineCommand {
            session: None,
            action: EngineAction::GetPlayerByPassphrase(passphrase),
        },
        Location(location) => EngineCommand {
            session: Some(session),
            action: EngineAction::Location {
                player: player_id,
                location,
            },
        },
        AttachImage {
            challenge_index: _,
            image: _,
        } => todo!(),
        Complete(id) => EngineCommand {
            session: Some(session),
            action: EngineAction::Complete {
                completer: team_id,
                completed: id,
            },
        },
        Catch(caught) => EngineCommand {
            session: Some(session),
            action: EngineAction::Catch {
                catcher: team_id,
                caught,
            },
        },
        Ping(mayssage) => EngineCommand {
            session: None,
            action: EngineAction::Ping(mayssage),
        },
        RequestEverything => EngineCommand {
            session: Some(session),
            action: EngineAction::GetState,
        },
    }
}

async fn handle_client(stream: TcpStream) -> Result<(), api::error::Error> {
    let (tcp_rx, tcp_tx) = stream.into_split();
    let mut transport_rx = FramedRead::new(tcp_rx, LengthDelimitedCodec::new());
    let mut transport_tx = FramedWrite::new(tcp_tx, LengthDelimitedCodec::new());

    let (mut truin_tx, truin_rx) = api::connect(None).await?;
    let (internal_tx, mut internal_rx) = mpsc::unbounded_channel();
    let internal_tx_2 = internal_tx.clone();

    // the following 56 lines are ugly as all hell, please help me
    async fn login_successful(
        tx: &mut FramedWrite<OwnedWriteHalf, LengthDelimitedCodec>,
        value: bool,
    ) {
        tx.send(
            bincode::serialize(&trainlappcomms::ToApp::LoginSuccessful(value))
                .unwrap()
                .into(),
        )
        .await
        .unwrap();
    }
    let (player_id, session, team_id) = loop {
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
                ResponseAction::Player(player) => {
                    if let Some(session) = player.session {
                        if let ResponseAction::SendState { teams, game: _ } = truin_tx
                            .send(EngineCommand {
                                session: Some(session),
                                action: EngineAction::GetState,
                            })
                            .await
                            .unwrap()
                        {
                            if let Some(team_id) = teams
                                .iter()
                                .position(|t| t.players.iter().any(|p| p.id == player.id))
                            {
                                login_successful(&mut transport_tx, true).await;
                                break (player.id, session, team_id);
                            }
                            login_successful(&mut transport_tx, false).await;
                        }
                        login_successful(&mut transport_tx, false).await;
                    } else {
                        login_successful(&mut transport_tx, false).await;
                    }
                }
                _ => {
                    login_successful(&mut transport_tx, false).await;
                }
            }
        }
    };

    let mut truin_tx_2 = truin_tx.clone();
    let app_receiver = async move {
        while let Some(message) = transport_rx.next().await {
            let message = message.unwrap();
            let message = bincode::deserialize::<trainlappcomms::ToServer>(&message).unwrap();
            let message = to_server_to_engine_command(message, session, team_id, player_id);
            match truin_tx_2.send(message).await {
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
            transport_tx
                .send(bincode::serialize(&message).unwrap().into())
                .await
                .unwrap();
        }
    };

    let truin_receiver = async move {
        let mut truin_rx = truin_rx.activate().await;
        loop {
            if let Some(message) = truin_rx.recv().await {
                internal_tx
                    .send(broadcast_to_to_app(message, player_id, &mut truin_tx).await)
                    .unwrap()
            }
        }
    };

    tokio::select! {
        _ = app_sender => (),
        _ = app_receiver => (),
        _ = truin_receiver => (),
    };
    Ok(())
}

#[tokio::main()]
async fn main() -> std::io::Result<()> {
    let listener = TcpListener::bind("192.168.50.69:41314").await?;
    println!("Server listening on port 41314");

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
