// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

use std::fmt::{Display, Formatter};

use serde::{Deserialize, Serialize};

use crate::cli::slack::SlackUser;

use super::notion::NotionPerson;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct User {
    pub(crate) slack_user: Option<SlackUser>,
    pub(crate) notion_user: Option<NotionPerson>,
}

impl User {
    pub fn new(slack_user: Option<SlackUser>, notion_user: Option<NotionPerson>) -> Option<User> {
        if slack_user.is_none() && notion_user.is_none() {
            None
        } else {
            Some(User {
                slack_user,
                notion_user,
            })
        }
    }

    /// Returns a string indicating which systems this user exists in
    pub fn system_presence(&self) -> String {
        let mut presence = Vec::new();
        if self.slack_user.is_some() {
            presence.push("Slack");
        }
        if self.notion_user.is_some() {
            presence.push("Notion");
        }
        presence.join(" & ")
    }
}

impl Display for User {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        let name = self
            .slack_user
            .as_ref()
            .map(|u| {
                format!(
                    "{} {}",
                    u.name.clone(),
                    u.profile
                        .as_ref()
                        .map(|p| format!("({})", p.email.as_ref().unwrap_or(&"".to_string())))
                        .unwrap_or("".to_string())
                )
            })
            .or_else(|| self.notion_user.as_ref().map(|u| u.name.clone()));
        if let Some(name) = name {
            write!(f, "{} [{}]", name, self.system_presence())
        } else {
            write!(
                f,
                "{} [{}]",
                self.notion_user
                    .as_ref()
                    .expect("expected notion user")
                    .name,
                self.system_presence()
            )
        }
    }
}
