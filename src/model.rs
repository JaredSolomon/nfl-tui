use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScoreboardResponse {
    pub events: Vec<Event>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Event {
    pub short_name: String,
    pub competitions: Vec<Competition>,
    pub status: Status,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Competition {
    pub competitors: Vec<Competitor>,
    pub status: Status,
    pub situation: Option<Situation>,
    pub broadcasts: Option<Vec<Broadcast>>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Broadcast {
    pub market: Option<String>,
    pub names: Vec<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Competitor {
    pub team: Team,
    pub score: Option<String>,
    pub home_away: String,
    pub winner: Option<bool>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Team {
    pub id: Option<String>,
    pub abbreviation: String,
    pub display_name: String,
    pub short_display_name: String,
    pub color: Option<String>,
    pub alternate_color: Option<String>,
    pub logo: Option<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub period: i32,
    pub display_clock: String,
    pub clock: Option<f64>,
    #[serde(rename = "type")]
    pub type_field: StatusType,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusType {
    pub state: String, // "pre", "in", "post"
    pub short_detail: String,
    pub description: String,
    pub detail: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Situation {
    pub down: Option<i32>,
    pub distance: Option<i32>,
    pub yard_line: Option<i32>,
    pub short_down_distance_text: Option<String>,
    pub possession: Option<String>,
    pub last_play: Option<LastPlay>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LastPlay {
    pub text: String,
}
