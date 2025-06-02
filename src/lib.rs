use serde::{Deserialize, Serialize};

#[cfg(feature = "build-binary")]
use chrono::Timelike;

pub mod api;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum ToServer {
    Login(String),
    Location(DetailedLocation),
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
    Location {
        team: usize,
        location: DetailedLocation,
    },
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
        picture_ids: Vec<u64>,
    },
    Complete {
        challenge: Challenge,
        completer_id: usize,
        time: u32,
        picture_ids: Vec<u64>,
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
                picture_ids,
            } => Event::Complete {
                challenge: challenge.into(),
                completer_id,
                time: time.num_seconds_from_midnight(),
                picture_ids,
            },
            truinlag::Event::Catch {
                catcher_id,
                caught_id,
                bounty,
                time,
                picture_ids,
            } => Event::CatchTeam {
                catcher_id,
                caught_id,
                bounty,
                time: time.num_seconds_from_midnight(),
                picture_ids,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetailedLocation {
    pub latitude: f32,
    pub longitude: f32,
    /// accuracy in metres
    pub accuracy: u16,
    /// heading in degrees. 0º is north (hopefully)
    pub heading: f32,
    /// speed in m/s
    pub speed: f32,
    pub timestamp: i64,
}

#[cfg(feature = "build-binary")]
impl Into<truinlag::DetailedLocation> for DetailedLocation {
    fn into(self) -> truinlag::DetailedLocation {
        truinlag::DetailedLocation {
            latitude: self.latitude,
            longitude: self.longitude,
            accuracy: self.accuracy,
            heading: self.heading,
            speed: self.speed,
            timestamp: self.timestamp,
        }
    }
}

#[cfg(feature = "build-binary")]
impl From<truinlag::DetailedLocation> for DetailedLocation {
    fn from(value: truinlag::DetailedLocation) -> Self {
        Self {
            latitude: value.latitude,
            longitude: value.longitude,
            accuracy: value.accuracy,
            heading: value.heading,
            speed: value.speed,
            timestamp: value.timestamp,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MinimalLocation {
    pub latitude: f32,
    pub longitude: f32,
    pub timestamp: i64,
}

#[cfg(feature = "build-binary")]
impl Into<truinlag::MinimalLocation> for MinimalLocation {
    fn into(self) -> truinlag::MinimalLocation {
        truinlag::MinimalLocation {
            latitude: self.latitude,
            longitude: self.longitude,
            timestamp: self.timestamp,
        }
    }
}

#[cfg(feature = "build-binary")]
impl From<truinlag::MinimalLocation> for MinimalLocation {
    fn from(value: truinlag::MinimalLocation) -> Self {
        Self {
            latitude: value.latitude,
            longitude: value.longitude,
            timestamp: value.timestamp,
        }
    }
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
    pub location: Option<DetailedLocation>,
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
            location: value.location.map(|l| l.into()),
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
    pub picture_id: Option<u64>,
}

#[cfg(feature = "build-binary")]
impl From<truinlag::Player> for Player {
    fn from(value: truinlag::Player) -> Self {
        Self {
            name: value.name,
            id: value.id,
            picture_id: value.picture_id,
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
