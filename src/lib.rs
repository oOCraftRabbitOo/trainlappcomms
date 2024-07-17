use chrono;
use serde::{Deserialize, Serialize};
use truinlag;

#[derive(Serialize, Deserialize)]
pub enum ToServer {
    Login(String),
    Location((f64, f64)),
    AttachImage {
        challenge_index: u64,
        image: truinlag::Jpeg,
    },
    Complete(u64),
    Catch(u64),
    RequestEverything,
    Ping(Option<String>),
}

#[derive(Serialize, Deserialize)]
pub struct Everything {
    pub state: State,
    pub teams: Vec<Team>,
    pub you: u64,
    pub your_team: usize,
}

#[derive(Serialize, Deserialize)]
pub enum ToApp {
    Everything(Everything),
    LoginSuccessful(bool),
    Ping(Option<String>),
    ToCatcher(Everything),
    ToRunner(Everything),
}

#[derive(Serialize, Deserialize)]
pub enum State {
    GameNotRunning,
    Runner,
    Catcher,
}

#[derive(Serialize, Deserialize)]
pub struct Team {
    pub is_catcher: truinlag::TeamRole,
    pub name: String,
    pub id: usize,
    pub bounty: u64,
    pub points: u64,
    pub players: Vec<Player>,
    pub challenges: Vec<Challenge>,
    pub completed_challenges: Vec<CompletedChallenge>,
    // pub thumb_name: String,
    pub location: (f64, f64),
}

impl From<truinlag::Team> for Team {
    fn from(value: truinlag::Team) -> Self {
        Self {
            is_catcher: value.role,
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
            location: value.location,
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct Challenge {
    pub title: String,
    pub description: String,
    pub points: u64,
    // pub attached_images: Vec<String>,
}

impl From<truinlag::Challenge> for Challenge {
    fn from(challenge: truinlag::Challenge) -> Self {
        Challenge {
            title: challenge.title,
            description: challenge.description,
            points: challenge.points,
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct CompletedChallenge {
    pub title: String,
    pub description: String,
    pub points: u64,
    pub time: chrono::NaiveTime,
    // pub attached_images: Vec<String>,
}

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

#[derive(Serialize, Deserialize)]
pub struct Player {
    pub name: String,
    pub id: u64,
    // pub thumb_name: String,
}

impl From<truinlag::Player> for Player {
    fn from(value: truinlag::Player) -> Self {
        Self {
            name: value.name,
            id: value.id,
        }
    }
}
