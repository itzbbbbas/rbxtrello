use anyhow::Context;
use reqwest::Method;
use serde::de::DeserializeOwned;

use crate::api::{credentials, http, model::*};

const TRELLO_API: &str = "https://api.trello.com/1";

async fn request<T: DeserializeOwned>(
    method: Method,
    path: &str,
    body_params: Option<&[(&str, &str)]>,
) -> anyhow::Result<T> {
    let creds = credentials().await?;
    let url = format!("{TRELLO_API}{path}");
    let mut req = http()
        .request(method.clone(), &url)
        .query(&[("key", &creds.key), ("token", &creds.token)]);

    if let Some(params) = body_params {
        req = req.form(params);
    }

    let resp = req
        .send()
        .await
        .with_context(|| format!("{} {}", method, path))?;
    let status = resp.status();
    let text = resp
        .text()
        .await
        .with_context(|| format!("reading body of {} {}", method, path))?;

    if !status.is_success() {
        anyhow::bail!("{} {} → {}: {}", method, path, status, text);
    }

    if text.is_empty() {
        // serde_json doesn't parse "" into ()
        return serde_json::from_str("null")
            .with_context(|| format!("empty body for {} {}", method, path));
    }

    serde_json::from_str(&text)
        .with_context(|| format!("decoding response of {} {}: {}", method, path, text))
}

// MARK: Boards

pub async fn create_board(name: &str) -> anyhow::Result<Board> {
    request(
        Method::POST,
        "/boards",
        Some(&[("name", name), ("defaultLists", "false")]),
    )
    .await
}

pub async fn get_board(board_id: &str) -> anyhow::Result<Board> {
    request(Method::GET, &format!("/boards/{board_id}"), None).await
}

// MARK: Lists

pub async fn list_board_lists(board_id: &str) -> anyhow::Result<Vec<List>> {
    request(Method::GET, &format!("/boards/{board_id}/lists"), None).await
}

pub async fn create_list(board_id: &str, name: &str, pos: &str) -> anyhow::Result<List> {
    request(
        Method::POST,
        "/lists",
        Some(&[("name", name), ("idBoard", board_id), ("pos", pos)]),
    )
    .await
}

pub async fn update_list(list_id: &str, name: &str, pos: &str) -> anyhow::Result<List> {
    request(
        Method::PUT,
        &format!("/lists/{list_id}"),
        Some(&[("name", name), ("pos", pos)]),
    )
    .await
}

pub async fn archive_list(list_id: &str) -> anyhow::Result<List> {
    request(
        Method::PUT,
        &format!("/lists/{list_id}/closed"),
        Some(&[("value", "true")]),
    )
    .await
}

// MARK: Cards

pub async fn list_cards(list_id: &str) -> anyhow::Result<Vec<Card>> {
    request(Method::GET, &format!("/lists/{list_id}/cards"), None).await
}

pub async fn create_card(
    list_id: &str,
    name: &str,
    desc: &str,
    label_ids: &[String],
) -> anyhow::Result<Card> {
    let labels = label_ids.join(",");
    let mut params: Vec<(&str, &str)> = vec![("idList", list_id), ("name", name), ("desc", desc)];
    if !label_ids.is_empty() {
        params.push(("idLabels", &labels));
    }
    request(Method::POST, "/cards", Some(&params)).await
}

pub async fn update_card(
    card_id: &str,
    name: &str,
    desc: &str,
    label_ids: &[String],
) -> anyhow::Result<Card> {
    let labels = label_ids.join(",");
    let params = vec![
        ("name", name),
        ("desc", desc),
        ("idLabels", labels.as_str()),
    ];
    request(Method::PUT, &format!("/cards/{card_id}"), Some(&params)).await
}

pub async fn archive_card(card_id: &str) -> anyhow::Result<Card> {
    request(
        Method::PUT,
        &format!("/cards/{card_id}"),
        Some(&[("closed", "true")]),
    )
    .await
}

// MARK: Labels

pub async fn list_board_labels(board_id: &str) -> anyhow::Result<Vec<Label>> {
    request(Method::GET, &format!("/boards/{board_id}/labels"), None).await
}

pub async fn create_label(board_id: &str, name: &str, color: &str) -> anyhow::Result<Label> {
    request(
        Method::POST,
        "/labels",
        Some(&[("name", name), ("color", color), ("idBoard", board_id)]),
    )
    .await
}

pub async fn update_label(label_id: &str, name: &str, color: &str) -> anyhow::Result<Label> {
    request(
        Method::PUT,
        &format!("/labels/{label_id}"),
        Some(&[("name", name), ("color", color)]),
    )
    .await
}
