// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use anyhow::anyhow;
use anyhow::Context;
use anyhow::Result;
use reqwest::Client;
use serde::Deserialize;
use serde::Serialize;
use std::fmt::Display;
use std::fmt::Formatter;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{debug, info};

const CHANNELS_URL: &str = "https://slack.com/api/conversations.list";

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct UsersResponse {
    ok: bool,
    members: Option<Vec<SlackUser>>,
    response_metadata: Option<ResponseMetadata>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct SlackUser {
    pub id: String,
    pub name: String,
    pub profile: Option<Profile>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Profile {
    pub email: Option<String>,
}

impl Display for SlackUser {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Channel {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct ResponseMetadata {
    next_cursor: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct ConversationsResponse {
    ok: bool,
    error: Option<String>,
    channels: Option<Vec<Channel>>,
    response_metadata: Option<ResponseMetadata>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct SendMessageBody {
    channel: String,
    text: String,
    ts: String,
    mrkdwn: bool,
}

pub async fn get_channels(client: &Client) -> Result<Vec<Channel>> {
    let mut channels: Vec<Channel> = vec![];

    let mut result: ConversationsResponse = client
        .get(CHANNELS_URL)
        .send()
        .await
        .map_err(|e| anyhow!(e))?
        .json()
        .await?;
    let new_channels = result
        .clone()
        .channels
        .unwrap_or_else(|| panic!("Expected channels to exist for {:?}", result))
        .clone();
    channels.extend(new_channels.into_iter());
    if result.response_metadata.is_none() {
        debug!("No pagination in channels response");
        return Ok(channels);
    }
    while let Some(cursor) = result
        .response_metadata
        .expect("Expected response metadata")
        .next_cursor
    {
        if cursor.is_empty() {
            break;
        }
        result = client
            .get(CHANNELS_URL)
            .query(&[("cursor", cursor)])
            .send()
            .await
            .map_err(|e| anyhow!(e))?
            .json()
            .await
            .context("parsing json from channels api")?;
        let extra_channels = result
            .clone()
            .channels
            .unwrap_or_else(|| panic!("Expected channels to exist for {:?}", result))
            .clone();
        channels.extend(extra_channels.into_iter());
    }
    channels = channels.iter().map(|c| (*c).clone()).collect();
    Ok(channels)
}

pub async fn get_users(client: &Client) -> Result<Vec<SlackUser>> {
    let url = "https://slack.com/api/users.list";
    let mut all_users = Vec::new();
    let mut cursor: Option<String> = None;
    let mut has_more = true;

    while has_more {
        let mut request = client.get(url);

        if let Some(ref cursor_value) = cursor {
            request = request.query(&[("cursor", cursor_value)]);
        }

        let response = request
            .send()
            .await
            .map_err(|e| anyhow!(e))?
            .json::<UsersResponse>()
            .await?;

        if !response.ok {
            return Err(anyhow::anyhow!("Failed to get users: API returned not OK"));
        }

        if let Some(members) = response.members {
            if *crate::DEBUG_MODE {
                info!("Retrieved {} users from Slack API", members.len());
            }
            all_users.extend(members);
        }

        // Check if there are more results
        if let Some(metadata) = response.response_metadata {
            if let Some(next_cursor) = metadata.next_cursor {
                if !next_cursor.is_empty() {
                    cursor = Some(next_cursor);
                    has_more = true;
                } else {
                    has_more = false;
                }
            } else {
                has_more = false;
            }
        } else {
            has_more = false;
        }
    }

    if *crate::DEBUG_MODE {
        info!("Total users retrieved from Slack: {}", all_users.len());
    }

    Ok(all_users)
}

pub async fn send_message(client: &Client, channel: &str, message: &str) -> Result<()> {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_millis();
    let message_body = SendMessageBody {
        channel: channel.to_owned(),
        text: message.to_owned(),
        ts: timestamp.to_string(),
        mrkdwn: true,
    };
    let url = "https://slack.com/api/chat.postMessage";
    let response = client.post(url).json(&message_body).send().await?;
    let response = response.json::<serde_json::Value>().await?;
    if response["ok"].as_bool().expect("ok was not a bool") {
        Ok(())
    } else {
        Err(anyhow!("Failed to send message: {}", response))
    }
}
