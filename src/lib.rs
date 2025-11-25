use std::num::NonZeroU32;

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
    Complete {
        completed_id: usize,
        period_id: usize,
    },
    Catch {
        caught_id: usize,
        period_id: usize,
    },
    RequestEverything,
    Ping(Option<String>),
    RequestPictures(Vec<u64>),
    RequestThumbnails(Vec<u64>),
    RequestPastLocations {
        of_past_seconds: Option<NonZeroU32>,
        team_id: usize,
    },
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
    BecomeCatcher(Everything), // I'm too lazy for notifications here
    BecomeRunner(Everything),
    ChallengeCompleted(Event, Everything),
    BecomeNoGameRunning(Everything),
    BecomeShutDown,
    Location {
        team: usize,
        location: DetailedLocation,
    },
    AddedPeriod(usize),
    Pictures(Vec<JuhuiPicture>),
    Error(ClientError),
    SendPastLocations {
        team: usize,
        locations: Vec<MinimalLocation>,
    },
    GameStarted(Everything),
    EventOccurred(Event, Everything),
    YouLeftGracePeriod(Everything),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum ClientError {
    NotFound(String),     // Element you were looking for wasn't found
    TeamExists(String),   // You cannot create a team if one with a similar name already exists
    AlreadyExists,        // Things that already exist cannot be created
    GameInProgress,       // Commands like AddTeam cannot be run if a game is in progress
    GameNotRunning,       // Commands like catch can only be run if a game is in progress
    AmbiguousData,        // If multiple matching objects exist, e.g. players with passphrase lol
    InternalError,        // Some sort of internal database error
    NotImplemented,       // Feature is not yet implemented
    TeamIsRunner(usize),  // A relevant team is runner, but has to be catcher
    TeamIsCatcher(usize), // A relevant team is catcher, but has to be runner
    TeamsTooFar,          // Two relevant teams are too far away from each other
    BadData(String),
    TextError(String), // Some other kind of error with a custom text
    PictureProblem,    // An Image-related error
    TooRapid,          // When requests are sent too rapidly
    TooFewChallenges,  // When there are too few challenges to start a game
}

impl std::fmt::Display for ClientError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound(ctx) => write!(f, "{} was not found", ctx),
            Self::TeamExists(team) => write!(f, "Team {} already exists", team),
            Self::AlreadyExists => write!(f, "Already exists"),
            Self::GameInProgress => write!(f, "There is already a game in progress"),
            Self::GameNotRunning => write!(f, "There is no game in progress"),
            Self::AmbiguousData => write!(f, "Ambiguous data"),
            Self::InternalError => write!(f, "There was a truinlag-internal error"),
            Self::NotImplemented => write!(f, "Not yet implemented"),
            Self::TeamIsRunner(team) => write!(f, "team {} is runner", team),
            Self::TeamIsCatcher(team) => write!(f, "team {} is catcher", team),
            Self::TeamsTooFar => write!(f, "the teams are too far away from each other"),
            Self::BadData(text) => write!(f, "bad data: {}", text),
            Self::TextError(text) => write!(f, "{}", text),
            Self::PictureProblem => write!(f, "there was a problem processing an image"),
            Self::TooRapid => write!(f, "not enough time has passed since the last request"),
            Self::TooFewChallenges => write!(
                f,
                "there are not enough challenges to start a game in the challenge db"
            ),
        }
    }
}

#[cfg(feature = "build-binary")]
impl TryFrom<truinlag::commands::Error> for ClientError {
    type Error = ();
    fn try_from(value: truinlag::commands::Error) -> Result<Self, Self::Error> {
        use truinlag::commands::Error::*;
        match value {
            NoSessionSupplied => Err(()),
            SessionSupplied => Err(()),
            NotFound(text) => Ok(Self::NotFound(text)),
            TeamExists(team) => Ok(Self::TeamExists(team)),
            AlreadyExists => Ok(Self::AlreadyExists),
            GameInProgress => Ok(Self::GameInProgress),
            GameNotRunning => Ok(Self::GameNotRunning),
            AmbiguousData => Ok(Self::AmbiguousData),
            InternalError => Ok(Self::InternalError),
            NotImplemented => Ok(Self::NotImplemented),
            TeamIsRunner(team) => Ok(Self::TeamIsRunner(team)),
            TeamIsCatcher(team) => Ok(Self::TeamIsCatcher(team)),
            TeamsTooFar => Ok(Self::TeamsTooFar),
            BadData(text) => Ok(Self::BadData(text)),
            TextError(text) => Ok(Self::TextError(text)),
            PictureProblem => Ok(Self::PictureProblem),
            TooRapid => Ok(Self::TooRapid),
            TooFewChallenges => Ok(Self::TooFewChallenges),
        }
    }
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
        location: MinimalLocation,
    },
    Complete {
        challenge: Challenge,
        completer_id: usize,
        time: u32,
        picture_ids: Vec<u64>,
        location: MinimalLocation,
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
                location,
            } => Event::Complete {
                challenge: challenge.into(),
                completer_id,
                time: time.num_seconds_from_midnight(),
                picture_ids,
                location: location.into(),
            },
            truinlag::Event::Catch {
                catcher_id,
                caught_id,
                bounty,
                time,
                picture_ids,
                location,
            } => Event::CatchTeam {
                catcher_id,
                caught_id,
                bounty,
                time: time.num_seconds_from_midnight(),
                picture_ids,
                location: location.into(),
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

#[derive(Serialize, Deserialize, Clone, Copy, Debug)]
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
impl From<DetailedLocation> for truinlag::DetailedLocation {
    fn from(val: DetailedLocation) -> Self {
        truinlag::DetailedLocation {
            latitude: val.latitude,
            longitude: val.longitude,
            accuracy: val.accuracy,
            heading: val.heading,
            speed: val.speed,
            timestamp: val.timestamp,
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
impl From<MinimalLocation> for truinlag::MinimalLocation {
    fn from(val: MinimalLocation) -> Self {
        truinlag::MinimalLocation {
            latitude: val.latitude,
            longitude: val.longitude,
            timestamp: val.timestamp,
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
    pub period_id: usize,
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
            period_id: value.period_id,
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
