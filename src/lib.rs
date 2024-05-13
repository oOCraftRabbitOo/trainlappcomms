use chrono;
use truinlag;

pub struct Team {
    pub name: String,
    pub bounty: u64,
    pub points: u64,
    pub challenges: Vec<Challenge>,
    pub completed_challenges: Vec<CompletedChallenge>,
    pub thumb: truinlag::Jpeg,
}

pub struct Challenge {
    pub title: String,
    pub description: String,
    pub points: u64,
}

pub struct CompletedChallenge {
    pub title: String,
    pub description: String,
    pub points: u64,
    pub time: chrono::NaiveTime,
}

pub struct Player {
    pub name: String,
    pub id: u64,
    pub thumb: truinlag::Jpeg,
}
