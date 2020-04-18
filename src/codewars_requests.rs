use crate::error::{CodewarsApiError, MainError};
use reqwest;
use serde::{Deserialize, Serialize};
use serde_json;

pub async fn get_honor(username: &str) -> Result<i64, MainError> {
    fn parse(user: CodewarsHonorResponse, username: &str) -> Result<User, MainError> {
        Ok(match user {
            CodewarsHonorResponse::Success(user) => Ok(user),
            CodewarsHonorResponse::Fail { reason, .. } if reason == "not found" => {
                Err(CodewarsApiError::NotFound(username.to_owned()))
            }
            CodewarsHonorResponse::Fail { reason, .. } => panic!("unknown error: {}", reason),
        }?)
    }
    let honor: CodewarsHonorResponse = serde_json::from_str(
        reqwest::get(&{
            let url = format!("https://www.codewars.com/api/v1/users/{}", username);
            log::info!("Request: {}", &url);
            url
        })
        .await?
        .text()
        .await?
        .as_str(),
    )?;

    Ok(parse(honor, username)?.honor)
}

pub async fn get_completed(username: &str) -> Result<Vec<CompletedKata>, MainError> {
    fn url(user: &str, page: i32) -> String {
        let url = format!(
            "https://www.codewars.com/api/v1/users/{}/code-challenges/completed?page={}",
            user, page
        );
        log::info!("Request: {}", &url);
        url
    }
    fn parse(pages: CodewarsResponse, username: &str) -> Result<CompletedKatas, MainError> {
        Ok(match pages {
            CodewarsResponse::Success(katas) => Ok(katas),
            CodewarsResponse::Fail { reason, .. } if reason == "not found" => {
                Err(CodewarsApiError::NotFound(username.to_owned()))
            }
            CodewarsResponse::Fail { reason, .. } => panic!("unknown error: {}", reason),
        }?)
    }

    let pages: CodewarsResponse = serde_json::from_str(
        reqwest::get(&url(username, 0))
            .await?
            .text()
            .await?
            .as_str(),
    )?;
    let mut pages = vec![parse(pages, username)?];

    for page in 1..pages.first().unwrap().total_pages {
        let new = parse(
            serde_json::from_str(
                reqwest::get(&url(username, page))
                    .await?
                    .text()
                    .await?
                    .as_str(),
            )?,
            username,
        )?;
        pages.push(new)
    }
    Ok(pages.into_iter().fold(Vec::new(), |mut acc, mut page| {
        acc.append(&mut page.data);
        acc
    }))
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(untagged)]
enum CodewarsHonorResponse {
    Fail { success: bool, reason: String },
    Success(User),
}

#[derive(Deserialize, Serialize, Debug)]
struct User {
    honor: i64,
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(untagged)]
enum CodewarsResponse {
    Fail { success: bool, reason: String },
    Success(CompletedKatas),
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
