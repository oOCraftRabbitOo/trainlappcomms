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
            let err = match err.try_into() {
                Ok(err) => err,
                Err(()) => {
                    return None;
                }
            };
            Some(ToApp::Error(err))
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
        SendLocations(_) => None,
        SendPastLocations { team_id, locations } => Some(ToApp::SendPastLocations {
            team: team_id,
            locations: locations.into_iter().map(|l| l.into()).collect(),
        }),
        SendGameConfig(_) => None,
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
        Location { team, location } => Some(ToApp::Location {
            team,
            location: location.into(),
        }),
        Caught { catcher, caught } => {
            let everything = get_everything(player_id, truin_tx, session).await;
            if catcher.id == team_id {
                Some(ToApp::BecomeRunner(everything))
            } else if caught.id == team_id {
                Some(ToApp::BecomeCatcher(everything))
            } else {
                Some(ToApp::EventOccurred(
                    Event::CatchTeam {
                        catcher_id: catcher.id,
                        caught_id: caught.id,
                        bounty: 0,
                        time: 0,
                        picture_ids: Vec::new(),
                        location: MinimalLocation {
                            latitude: 0_f32,
                            longitude: 0_f32,
                            timestamp: 0,
                        },
                    },
                    everything,
                ))
            }
        }
        Completed {
            completer,
            completed,
        } => {
            let everything = get_everything(player_id, truin_tx, session).await;
            let event = Event::Complete {
                challenge: completed.into(),
                completer_id: completer.id,
                time: 0,
                picture_ids: Vec::new(),
                location: MinimalLocation {
                    latitude: 0_f32,
                    longitude: 0_f32,
                    timestamp: 0,
                },
            };
            if completer.id == team_id {
                Some(ToApp::ChallengeCompleted(event, everything))
            } else {
                Some(ToApp::EventOccurred(event, everything))
            }
        }
        Pinged(mayssage) => Some(ToApp::Ping(mayssage)),
        Ended => Some(ToApp::BecomeNoGameRunning(
            get_everything(player_id, truin_tx, session).await,
        )),
        Started { teams: _, game: _ } => Some(ToApp::GameStarted(
            get_everything(player_id, truin_tx, session).await,
        )),
        TeamLeftGracePeriod(team) => {
            let everything = get_everything(player_id, truin_tx, session).await;
            if team.id == team_id {
                Some(ToApp::YouLeftGracePeriod(everything))
            } else {
                Some(ToApp::Everything(everything))
            }
        }
        PlayerChangedSession {
            player,
            from_session: _,
            to_session: _,
        } => {
            if player.id == player_id {
                panic!(
                    "received signal from truinlag \
                    indicating a player session change, \
                    panicking and hoping for reconnect :)"
                )
            } else {
                None
            }
        }
        PlayerChangedTeam {
            session: _,
            player,
            from_team: _,
            to_team: _,
        } => {
            if player == player_id {
                panic!(
                    "received signal from truinlag \
                    indicating a player team change, \
                    panicking and hoping for reconnect :)"
                )
            } else {
                None
            }
        }
        PlayerDeleted(player) => {
            if player.id == player_id {
                panic!(
                    "received signal from truinlag \
                    indicating that the player has been deleted, \
                    panicking and shutting down"
                )
            } else {
                None
            }
        }
    }
}

enum EngineCommandConversion {
    Instant(Box<EngineCommand>),
    Delayed(std::pin::Pin<Box<dyn Future<Output = EngineCommand> + Send + Sync>>),
}

impl From<EngineCommand> for EngineCommandConversion {
    fn from(value: EngineCommand) -> Self {
        Self::Instant(Box::new(value))
    }
}

fn to_server_to_engine_command(
    to_server: ToServer,
    session: u64,
    team_id: usize,
    player_id: u64,
) -> EngineCommandConversion {
    use ToServer::*;
    match to_server {
        Login(passphrase) => EngineCommand {
            session: None,
            action: EngineAction::GetPlayerByPassphrase(passphrase),
        }
        .into(),
        Location(location) => EngineCommand {
            session: Some(session),
            action: EngineAction::SendLocation {
                player: player_id,
                location: location.into(),
            },
        }
        .into(),
        AttachPeriodPictures { event_id, pictures } => {
            EngineCommandConversion::Delayed(Box::pin(async move {
                let pictures = tokio::task::block_in_place(|| {
                    pictures
                        .into_iter()
                        .filter_map(|p| RawPicture::from_bytes(p).ok())
                        .collect()
                });
                EngineCommand {
                    session: Some(session),
                    action: EngineAction::UploadPeriodPictures {
                        pictures,
                        team: team_id,
                        period: event_id,
                    },
                }
            }))
        }
        UploadPlayerPicture(picture) => EngineCommandConversion::Delayed(Box::pin(async move {
            let picture = tokio::task::block_in_place(|| RawPicture::from_bytes(picture).unwrap());
            EngineCommand {
                session: None,
                action: EngineAction::UploadPlayerPicture { player_id, picture },
            }
        })),
        UploadTeamPicture(picture) => EngineCommandConversion::Delayed(Box::pin(async move {
            let picture = tokio::task::block_in_place(|| RawPicture::from_bytes(picture).unwrap());
            EngineCommand {
                session: Some(session),
                action: EngineAction::UploadTeamPicture { team_id, picture },
            }
        })),
        Complete {
            completed_id,
            period_id,
        } => EngineCommand {
            session: Some(session),
            action: EngineAction::Complete {
                completer: team_id,
                completed: completed_id,
                period_id,
            },
        }
        .into(),
        Catch {
            caught_id,
            period_id,
        } => EngineCommand {
            session: Some(session),
            action: EngineAction::Catch {
                catcher: team_id,
                caught: caught_id,
                period_id,
            },
        }
        .into(),
        Ping(mayssage) => EngineCommand {
            session: None,
            action: EngineAction::Ping(mayssage),
        }
        .into(),
        RequestEverything => EngineCommand {
            session: Some(session),
            action: EngineAction::GetState,
        }
        .into(),
        RequestPictures(pictures) => EngineCommand {
            session: None,
            action: EngineAction::GetPictures(pictures),
        }
        .into(),
        RequestThumbnails(thumbnails) => EngineCommand {
            session: None,
            action: EngineAction::GetThumbnails(thumbnails),
        }
        .into(),
        RequestPastLocations {
            of_past_seconds,
            team_id,
        } => EngineCommand {
            session: Some(session),
            action: EngineAction::GetPastLocations {
                of_past_seconds,
                team_id,
            },
        }
        .into(),
    }
}

async fn handle_client(stream: TcpStream) -> Result<(), api::error::Error> {
    let (tcp_rx, tcp_tx) = stream.into_split();
    let mut transport_rx = FramedRead::new(tcp_rx, LengthDelimitedCodec::new());
    let mut transport_tx = FramedWrite::new(tcp_tx, LengthDelimitedCodec::new());

    let socket = format!(
        "/tmp/truinsocket_{}{}",
        if cfg!(debug_assertions) { "dev_" } else { "" },
        env!("CARGO_PKG_VERSION")
    );

    let (mut truin_tx, truin_rx) = api::connect(Some(&socket)).await?;
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
        truin_sender_tx: mpsc::UnboundedSender<EngineCommand>,
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
            match to_server_to_engine_command(message, session, team_id, player_id) {
                EngineCommandConversion::Instant(command) => truin_sender_tx.send(*command)?,
                EngineCommandConversion::Delayed(future) => {
                    let tx = truin_sender_tx.clone();
                    tokio::spawn(async move { tx.send(future.await).unwrap() });
                }
            };
            count += 1;
        }
        eprintln!("Stream returned None, client probably disconnected");
        Ok(())
    }

    let (truin_sender_tx, truin_sender_rx) = mpsc::unbounded_channel();
    let app_receiver = app_receiver(transport_rx, truin_sender_tx, session, team_id, player_id);

    async fn truin_sender(
        mut rx: mpsc::UnboundedReceiver<EngineCommand>,
        mut truin_tx_2: api::SendConnection,
        internal_tx_2: mpsc::UnboundedSender<ToApp>,
        player_id: u64,
        session: u64,
    ) -> Result<(), Box<dyn Error>> {
        while let Some(command) = rx.recv().await {
            match truin_tx_2.send(command).await {
                Ok(response) => {
                    if let Some(response) = response_to_to_app(response, player_id, session) {
                        internal_tx_2.send(response)?
                    }
                }
                Err(err) => {
                    eprintln!("error sending to truinlag, stopping truin_sender: {}", err);
                    break;
                }
            }
        }
        Ok(())
    }

    let truin_tx_2 = truin_tx.clone();

    let truin_sender = truin_sender(
        truin_sender_rx,
        truin_tx_2,
        internal_tx_2,
        player_id,
        session,
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
        res = truin_sender => res,
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
    let listener = TcpListener::bind(if cfg!(debug_assertions) {
        "192.168.1.125:42314"
    } else {
        "192.168.1.125:41314"
    })
    .await?;
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
    let listener = TcpListener::bind(if cfg!(debug_assertions) {
        "192.168.1.125:42315"
    } else {
        "192.168.1.125:41315"
    })
    .await?;
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
