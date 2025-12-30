use crate::model::ScoreboardResponse;
use anyhow::Result;
use reqwest::Client;

pub struct DataClient {
    client: Client,
}

impl DataClient {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }

    pub async fn fetch_scoreboard(&self, league: &str) -> Result<ScoreboardResponse> {
        let url = format!("https://site.api.espn.com/apis/site/v2/sports/football/{}/scoreboard", league);
        let resp = self.client.get(&url).send().await?;
        let data = resp.json::<ScoreboardResponse>().await?;
        Ok(data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_deserialize_sample() {
        // Read the sample file created earlier
        let content = fs::read_to_string("espn_data.json").expect("Failed to read sample file");
        let _data: ScoreboardResponse = serde_json::from_str(&content).expect("Failed to deserialize");
    }
}
