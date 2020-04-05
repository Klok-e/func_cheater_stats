use crate::error::MainError;
use reqwest;
use serde::{Deserialize, Serialize};
use serde_json;

pub async fn get_completed(username: &str) -> Result<Vec<CompletedKata>, MainError> {
    fn url(user: &str, page: i32) -> String {
        format!(
            "https://www.codewars.com/api/v1/users/{}/code-challenges/completed?page={}",
            user, page
        )
    }

    let mut pages: Vec<CompletedKatas> = vec![serde_json::from_str(
        reqwest::get(url(username, 0)).await?.text().await?.as_str(),
    )?];
    for page in 1..pages.first().unwrap().total_pages {
        let new = serde_json::from_str(
            reqwest::get(url(username, page))
                .await?
                .text()
                .await?
                .as_str(),
        )?;
        pages.push(new)
    }
    Ok(pages.into_iter().fold(Vec::new(), |mut acc, mut page| {
        acc.append(&mut page.data);
        acc
    }))
}

#[derive(Deserialize, Serialize, Debug)]
struct CompletedKatas {
    #[serde(rename = "totalPages")]
    total_pages: i32,
    data: Vec<CompletedKata>,
}
#[derive(Deserialize, Serialize, Debug)]
pub struct CompletedKata {
    pub name: String,
    #[serde(rename = "completedLanguages")]
    pub completed_languages: Vec<String>,
}
