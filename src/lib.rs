use chrono::Timelike;
use serde::{Deserialize, Serialize};

pub mod api;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum ToServer {
    Login(String),
    Location((f64, f64)),
    AttachPeriodPictures {
        event_id: usize,
        pictures: Vec<Vec<u8>>,
    },
    UploadPlayerPicture(Vec<u8>),
    UploadTeamPicture(Vec<u8>),
    Complete(usize),
    Catch(usize),
    RequestEverything,
    Ping(Option<String>),
    RequestPictures(Vec<u64>),
    RequestThumbnails(Vec<u64>),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ToServerPackage {
    contents: ToServer,
    id: Option<u64>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Everything {
    pub state: State,
    pub teams: Vec<Team>,
    pub events: Vec<Event>,
    pub you: u64,
    pub your_team: usize,
    pub your_session: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum ToApp {
    Everything(Everything),
    LoginSuccessful(bool),
    Ping(Option<String>),
    BecomeCatcher(Everything),
    BecomeRunner(Everything),
    BecomeShutDown,
    Location { team: usize, location: (f64, f64) },
    AddedPeriod(usize),
    Pictures(Vec<JuhuiPicture>),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ToAppPackage {
    contents: ToApp,
    id: Option<u64>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum Event {
    CatchTeam {
        catcher_id: usize,
        caught_id: usize,
        bounty: u64,
        time: u32,
    },
    Complete {
        challenge: Challenge,
        completer_id: usize,
        time: u32,
    },
}

#[cfg(feature = "build-binary")]
impl From<truinlag::Event> for Event {
    fn from(value: truinlag::Event) -> Self {
        match value {
            truinlag::Event::Complete {
                challenge,
                completer_id,
                time,
            } => Event::Complete {
                challenge: challenge.into(),
                completer_id,
                time: time.num_seconds_from_midnight(),
            },
            truinlag::Event::Catch {
                catcher_id,
                caught_id,
                bounty,
                time,
            } => Event::CatchTeam {
                catcher_id,
                caught_id,
                bounty,
                time: time.num_seconds_from_midnight(),
            },
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct JuhuiPicture {
    pub data: Vec<u8>,
    pub is_thumbnail: bool,
    pub id: u64,
}

#[cfg(feature = "build-binary")]
impl From<truinlag::Picture> for JuhuiPicture {
    fn from(value: truinlag::Picture) -> Self {
        JuhuiPicture {
            data: value.data.get_bytes(),
            is_thumbnail: value.is_thumbnail,
            id: value.id,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum State {
    GameNotRunning,
    Runner,
    Catcher,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Team {
    pub is_catcher: bool,
    pub name: String,
    pub picture_id: Option<u64>,
    pub id: usize,
    pub bounty: u64,
    pub points: u64,
    pub players: Vec<Player>,
    pub challenges: Vec<Challenge>,
    pub completed_challenges: Vec<CompletedChallenge>,
    pub colour: (u8, u8, u8),
    pub location: (f64, f64),
    pub in_grace_period: bool,
}

#[cfg(feature = "build-binary")]
impl From<truinlag::Team> for Team {
    fn from(value: truinlag::Team) -> Self {
        Self {
            colour: (value.colour.r, value.colour.g, value.colour.b),
            is_catcher: matches!(value.role, truinlag::TeamRole::Catcher),
            name: value.name,
            id: value.id,
            picture_id: value.picture_id,
            bounty: value.bounty,
            points: value.points,
            players: value.players.iter().map(|p| p.clone().into()).collect(),
            challenges: value.challenges.iter().map(|c| c.clone().into()).collect(),
            completed_challenges: value
                .completed_challenges
                .iter()
                .map(|cc| cc.clone().into())
                .collect(),
            location: value
                .location
                .unwrap_or((47.64984858748811, 8.570193667489143)),
            in_grace_period: value.in_grace_period,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Challenge {
    pub title: String,
    pub description: String,
    pub points: u64,
    // pub attached_images: Vec<String>,
}

#[cfg(feature = "build-binary")]
impl From<truinlag::Challenge> for Challenge {
    fn from(challenge: truinlag::Challenge) -> Self {
        Challenge {
            title: challenge.title,
            description: challenge.description,
            points: challenge.points,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CompletedChallenge {
    pub picture_ids: Vec<u64>,
    pub title: String,
    pub description: String,
    pub points: u64,
    pub time: chrono::NaiveTime,
    // pub attached_images: Vec<String>,
}

#[cfg(feature = "build-binary")]
impl From<truinlag::CompletedChallenge> for CompletedChallenge {
    fn from(value: truinlag::CompletedChallenge) -> Self {
        Self {
            picture_ids: value.picture_ids,
            title: value.title,
            description: value.description,
            points: value.points,
            time: value.time,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Player {
    pub name: String,
    pub id: u64,
    // pub thumb_name: String,
}

#[cfg(feature = "build-binary")]
impl From<truinlag::Player> for Player {
    fn from(value: truinlag::Player) -> Self {
        Self {
            name: value.name,
            id: value.id,
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PictureWrapper {
    pub kind: PictureKind,
    pub picture: Vec<u8>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum PictureKind {
    TeamProfile {
        session: u64,
        team: usize,
    },
    PlayerProfile(u64),
    Period {
        session: u64,
        team: usize,
        period_id: usize,
    },
}
