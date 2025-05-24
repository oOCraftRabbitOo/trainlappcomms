#![cfg(feature = "build-binary")]

use futures::prelude::*;
use std::error::Error;
use tokio::io::AsyncReadExt;
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;
use tokio_util::codec::{FramedRead, FramedWrite, LengthDelimitedCodec};
use trainlappcomms::*;
use truinlag::commands::{BroadcastAction, EngineAction, EngineCommand, ResponseAction};
use truinlag::TeamRole;
use truinlag::{api, RawPicture};

async fn get_everything(
    player_id: u64,
    truin_tx: &mut api::SendConnection,
    session: u64,
) -> Everything {
    match response_to_to_app(
        truin_tx
            .send(EngineCommand {
                session: Some(session),
                action: EngineAction::GetState,
            })
            .await
            .unwrap(),
        player_id,
        session,
    )
    .unwrap()
    {
        ToApp::Everything(everything) => everything,
        _ => panic!(),
    }
}

fn response_to_to_app(response: ResponseAction, player_id: u64, session_id: u64) -> Option<ToApp> {
    use ResponseAction::*;
    match response {
        Error(err) => {
            eprintln!("{}", err);
            None
        }
        Team(_) => None,
        Player(_) => None,
        Success => None,
        SendState {
            teams,
            events,
            game,
        } => {
            let your_team = teams
                .iter()
                .position(|t| t.players.iter().any(|p| p.id == player_id))
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
                events: events.into_iter().map(|e| e.into()).collect(),
                you: player_id,
                your_team,
                your_session: session_id,
            }))
        }
        SendGlobalState {
            sessions: _,
            players: _,
        } => None,
        SendRawChallenges(_) => None,
        SendChallengeSets(_) => None,
        SendZones(_) => None,
        SendEvents(_) => None,
        UploadedPictures(_) => None,
        Period(id) => Some(ToApp::AddedPeriod(id)),
        Pictures(pics) => Some(ToApp::Pictures(
            pics.into_iter().map(|p| p.into()).collect(),
        )),
    }
}

async fn broadcast_to_to_app(
    broadcast: BroadcastAction,
    player_id: u64,
    truin_tx: &mut api::SendConnection,
    session: u64,
    team_id: usize,
) -> Option<ToApp> {
    use BroadcastAction::*;
    match broadcast {
        TeamMadeRunner(team) => {
            if team.players.iter().any(|p| p.id == player_id) {
                let everything = get_everything(player_id, truin_tx, session).await;
                Some(ToApp::BecomeRunner(everything))
            } else {
                None
            }
        }
        TeamMadeCatcher(team) => {
            if team.id == team_id {
                let everything = get_everything(player_id, truin_tx, session).await;
                Some(ToApp::BecomeCatcher(everything))
            } else {
                None
            }
        }
        Location { team, location } => Some(ToApp::Location { team, location }),
        Caught { catcher, caught } => {
            let everything = get_everything(player_id, truin_tx, session).await;
            if catcher.id == team_id {
                return Some(ToApp::BecomeRunner(everything));
            } else if caught.id == team_id {
                return Some(ToApp::BecomeCatcher(everything));
            };
            Some(ToApp::Everything(everything))
        }
        Completed {
            completer,
            completed: _,
        } => {
            let everything = get_everything(player_id, truin_tx, session).await;
            if completer.id == team_id {
                Some(ToApp::BecomeRunner(everything))
            } else {
                Some(ToApp::Everything(everything))
            }
        }
        Pinged(mayssage) => Some(ToApp::Ping(mayssage)),
        Ended => Some(ToApp::BecomeShutDown),
        _ => todo!(),
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
            action: EngineAction::SendLocation {
                player: player_id,
                location,
            },
        },
        // AttachPeriodPictures { event_id, pictures } => EngineCommand {
        //     session: Some(session),
        //     action: EngineAction::UploadPeriodPictures {
        //         pictures: pictures
        //             .into_iter()
        //             .filter_map(|p| RawPicture::from_bytes(p).ok())
        //             .collect(),
        //         team: team_id,
        //         period: event_id,
        //     },
        // },
        // UploadPlayerPicture(picture) => EngineCommand {
        //     session: None,
        //     action: EngineAction::UploadPlayerPicture {
        //         player_id,
        //         picture: RawPicture::from_bytes(picture).unwrap(),
        //     },
        // },
        // UploadTeamPicture(picture) => EngineCommand {
        //     session: Some(session),
        //     action: EngineAction::UploadTeamPicture {
        //         team_id,
        //         picture: RawPicture::from_bytes(picture).unwrap(),
        //     },
        // },
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
        RequestPictures(pictures) => EngineCommand {
            session: None,
            action: EngineAction::GetPictures(pictures),
        },
        RequestThumbnails(thumbnails) => EngineCommand {
            session: None,
            action: EngineAction::GetThumbnails(thumbnails),
        },
    }
}

async fn handle_client(stream: TcpStream) -> Result<(), api::error::Error> {
    let (tcp_rx, tcp_tx) = stream.into_split();
    let mut transport_rx = FramedRead::new(tcp_rx, LengthDelimitedCodec::new());
    let mut transport_tx = FramedWrite::new(tcp_tx, LengthDelimitedCodec::new());

    let (mut truin_tx, truin_rx) = api::connect(None).await?;
    let (internal_tx, internal_rx) = mpsc::unbounded_channel();
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
            println!("TLC: App trying to connect with passphrase {}", passphrase);
            match truin_tx
                .send(EngineCommand {
                    session: None,
                    action: EngineAction::GetPlayerByPassphrase(passphrase),
                })
                .await
                .unwrap()
            {
                ResponseAction::Player(player) => {
                    println!("TLC: Player {} found in database", player.name);
                    if let Some(session) = player.session {
                        if let ResponseAction::SendState {
                            teams,
                            events: _,
                            game: _,
                        } = truin_tx
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
                                println!(
                                    "TLC: Player {} found in a team, login success",
                                    player.name
                                );
                                login_successful(&mut transport_tx, true).await;
                                break (player.id, session, team_id);
                            }
                            println!("TLC: Player {} not found in a team", player.name);
                            login_successful(&mut transport_tx, false).await;
                        }
                        println!("TLC: Couldn't get state from truinlag?!??!!");
                        login_successful(&mut transport_tx, false).await;
                    } else {
                        println!("TLC: Player {} has no session", player.name);
                        login_successful(&mut transport_tx, false).await;
                    }
                }
                _ => {
                    println!("TLC: Player not found or found multiple times");
                    login_successful(&mut transport_tx, false).await;
                }
            }
        } else {
            println!("received message from app that wasn't Login");
        }
    };

    async fn app_receiver(
        mut transport_rx: FramedRead<OwnedReadHalf, LengthDelimitedCodec>,
        mut truin_tx_2: api::SendConnection,
        internal_tx_2: mpsc::UnboundedSender<ToApp>,
        session: u64,
        team_id: usize,
        player_id: u64,
    ) -> Result<(), Box<dyn Error>> {
        let mut count: u64 = 0;
        while let Some(message) = transport_rx.next().await {
            println!("({}) received message from app", count);
            let message = message?;
            let message = bincode::deserialize::<trainlappcomms::ToServer>(&message).unwrap();
            //println!("({}) message: {:?}", count, message);
            let message = to_server_to_engine_command(message, session, team_id, player_id);
            // println!(
            //     "({}) parsed message, sending to truinlag: {:?}",
            //     count,
            //     message
            // );
            match truin_tx_2.send(message).await {
                Ok(response) => {
                    //println!("({}) received answer from truinlag: {:?}", count, response);
                    if let Some(response) = response_to_to_app(response, player_id, session) {
                        //println!("({}) parsed message, sending to app: {:?}", count, response);
                        internal_tx_2.send(response)?
                    }
                }
                Err(_) => break,
            }
            count += 1;
        }
        eprintln!("Stream returned None, client probably disconnected");
        Ok(())
    }

    let truin_tx_2 = truin_tx.clone();
    let app_receiver = app_receiver(
        transport_rx,
        truin_tx_2,
        internal_tx_2,
        session,
        team_id,
        player_id,
    );

    async fn app_sender(
        mut internal_rx: mpsc::UnboundedReceiver<ToApp>,
        mut transport_tx: FramedWrite<OwnedWriteHalf, LengthDelimitedCodec>,
    ) -> Result<(), Box<dyn Error>> {
        loop {
            let message = internal_rx.recv().await.ok_or("fuck")?;
            transport_tx
                .send(bincode::serialize(&message)?.into())
                .await?;
        }
    }

    let app_sender = app_sender(internal_rx, transport_tx);

    async fn truin_receiver(
        truin_rx: api::InactiveRecvConnection,
        internal_tx: mpsc::UnboundedSender<ToApp>,
        player_id: u64,
        mut truin_tx: api::SendConnection,
        session: u64,
        team_id: usize,
    ) -> Result<(), Box<dyn Error>> {
        let mut truin_rx = truin_rx.activate().await;
        loop {
            if let Some(message) = truin_rx.recv().await {
                let to_app =
                    broadcast_to_to_app(message, player_id, &mut truin_tx, session, team_id).await;
                if let Some(to_app) = to_app {
                    internal_tx.send(to_app)?
                }
            }
        }
    }

    let truin_receiver =
        truin_receiver(truin_rx, internal_tx, player_id, truin_tx, session, team_id);

    let res = tokio::select! {
        res = app_sender => res,
        res = app_receiver => res,
        res = truin_receiver => res,
    };
    match res {
        Ok(_) => println!("Client disconnected"),
        Err(err) => eprintln!("error occurred: {}", err),
    }
    Ok(())
}

#[tokio::main()]
async fn main() -> std::io::Result<()> {
    tokio::spawn(receive_picture_connections());
    let listener = TcpListener::bind("192.168.1.125:41314").await?;
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

async fn receive_picture_connections() -> std::io::Result<()> {
    let listener = TcpListener::bind("192.168.1.125:41315").await?;
    println!("Server listening for pictures on port 41315");

    loop {
        let accepted = listener.accept().await;
        match accepted {
            Ok((stream, addr)) => {
                println!("A picture client connected from {}", addr);
                tokio::spawn(handle_pictures(stream));
            }
            Err(e) => {
                eprintln!("Picture connection failed: {}", e);
            }
        }
    }
}

async fn handle_pictures(mut stream: TcpStream) {
    let mut buf = Vec::new();
    stream.read_to_end(&mut buf).await.unwrap();
    let pic = bincode::deserialize::<PictureWrapper>(&buf).unwrap();
    let kind = pic.kind;
    let pic = RawPicture::from_bytes(pic.picture).unwrap();
    let (mut truin_tx, _truin_rx) = api::connect(None).await.unwrap();
    match kind {
        PictureKind::TeamProfile { session, team } => {
            println!(
                "{:?}",
                truin_tx
                    .send(EngineCommand {
                        session: Some(session),
                        action: EngineAction::UploadTeamPicture {
                            team_id: team,
                            picture: pic
                        }
                    })
                    .await
                    .unwrap()
            )
        }
        PictureKind::PlayerProfile(player_id) => {
            println!(
                "{:?}",
                truin_tx
                    .send(EngineCommand {
                        session: None,
                        action: EngineAction::UploadPlayerPicture {
                            player_id,
                            picture: pic
                        }
                    })
                    .await
                    .unwrap()
            )
        }
        PictureKind::Period {
            session,
            team,
            period_id,
        } => {
            println!(
                "{:?}",
                truin_tx
                    .send(EngineCommand {
                        session: Some(session),
                        action: EngineAction::UploadPeriodPictures {
                            pictures: vec![pic],
                            team,
                            period: period_id
                        }
                    })
                    .await
                    .unwrap()
            )
        }
    }
}
