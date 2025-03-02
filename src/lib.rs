use serde::{Deserialize, Serialize};

pub mod api;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum ToServer {
    Login(String),
    Location((f64, f64)),
    // AttachImage {
    //     challenge_index: u64,
    //     image: truinlag::Jpeg,
    // },
    Complete(usize),
    Catch(usize),
    RequestEverything,
    Ping(Option<String>),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Everything {
    pub state: State,
    pub teams: Vec<Team>,
    pub events: Vec<Event>,
    pub you: u64,
    pub your_team: usize,
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
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum Event {
    Catch {
        catcher_id: usize,
        caught_id: usize,
        bounty: u64,
        time: chrono::NaiveTime,
    },
    Complete {
        challenge: Challenge,
        completer_id: usize,
        time: chrono::NaiveTime,
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
                time,
            },
            truinlag::Event::Catch {
                catcher_id,
                caught_id,
                bounty,
                time,
            } => Event::Catch {
                catcher_id,
                caught_id,
                bounty,
                time,
            },
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
    pub id: usize,
    pub bounty: u64,
    pub points: u64,
    pub players: Vec<Player>,
    pub challenges: Vec<Challenge>,
    pub completed_challenges: Vec<CompletedChallenge>,
    pub colour: (u8, u8, u8),
    // pub thumb_name: String,
    pub location: (f64, f64),
}

#[cfg(feature = "build-binary")]
impl From<truinlag::Team> for Team {
    fn from(value: truinlag::Team) -> Self {
        Self {
            colour: (value.colour.r, value.colour.g, value.colour.b),
            is_catcher: matches!(value.role, truinlag::TeamRole::Catcher),
            name: value.name,
            id: value.id,
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
