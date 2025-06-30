// Copyright 2025 Felipe Torres González
//
//    Licensed under the Apache License, Version 2.0 (the "License");
//    you may not use this file except in compliance with the License.
//    You may obtain a copy of the License at
//
//        http://www.apache.org/licenses/LICENSE-2.0
//
//    Unless required by applicable law or agreed to in writing, software
//    distributed under the License is distributed on an "AS IS" BASIS,
//    WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
//    See the License for the specific language governing permissions and
//    limitations under the License.

//! `Subscriptions` module.
//!
//! # Description
//!
//! This module features the `struct` **Subscriptions** which is in charge of handling what tickers is a client of the
//! bot subscribed to.
//!
//! The `struct` is a simple container of *Tickers*, as of today, simply 4 character strings. This invariant of
//! **Subscriptions** expects 4 character strings, thus it does not perform any kind of checks. The client of this
//! object must ensure that strings compliant with this rule are passed to the methods.
//!
//! The main goal of this `struct` is the abstraction of what a subscription is. This way, future changes won't break
//! clients of this module.

use crate::ClientError;
use std::{collections::HashSet, iter::IntoIterator};

const CHARS_PER_TICKER: u8 = 4;

#[derive(Clone, Default, Debug, PartialEq, Eq)]
pub struct Subscriptions {
    tickers: HashSet<String>,
}

impl std::ops::AddAssign for Subscriptions {
    fn add_assign(&mut self, rhs: Self) {
        rhs.tickers.iter().for_each(|e| {
            self.tickers.insert(e.clone());
        });
    }
}

impl std::ops::AddAssign<&Self> for Subscriptions {
    fn add_assign(&mut self, rhs: &Self) {
        rhs.tickers.iter().for_each(|e| {
            self.tickers.insert(e.clone());
        });
    }
}

impl std::ops::Sub for &Subscriptions {
    type Output = Subscriptions;

    fn sub(self, rhs: Self) -> Self::Output {
        let new_set = &self.tickers - &rhs.tickers;

        Self::Output { tickers: new_set }
    }
}

impl std::ops::SubAssign for Subscriptions {
    fn sub_assign(&mut self, other: Self) {
        other.tickers.iter().for_each(|e| {
            self.tickers.remove(e);
        });
    }
}

impl std::ops::SubAssign<&Self> for Subscriptions {
    fn sub_assign(&mut self, other: &Self) {
        other.tickers.iter().for_each(|e| {
            self.tickers.remove(e);
        });
    }
}

impl Subscriptions {
    /// Add a new subscription identified by a series of tickers.
    ///
    /// # Description
    ///
    /// Repeated tickers are ignored.
    pub fn add_subscriptions(&mut self, tickers: &[&str]) {
        tickers.iter().for_each(|t| {
            self.tickers.insert(String::from(*t));
        });
    }

    /// Remove a ticker from the subscription list.
    pub fn remove_subscriptions(&mut self, tickers: &[&str]) {
        tickers.iter().for_each(|t| {
            self.tickers.remove(*t);
        });
    }

    /// Check if the ticker(s) is already subscribed.
    ///
    /// # Description
    ///
    /// If more than one ticker are passed, the method will only return `true` when all the tickers are subscribed.
    pub fn is_subscribed(&self, tickers: &[&str]) -> bool {
        tickers
            .iter()
            .filter(|e| self.tickers.contains(**e))
            .collect::<Vec<_>>()
            .len()
            == tickers.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tickers.is_empty()
    }
}

impl IntoIterator for Subscriptions {
    type Item = String;
    type IntoIter = std::collections::hash_set::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.tickers.into_iter()
    }
}

impl<'a> IntoIterator for &'a Subscriptions {
    type Item = &'a String;
    type IntoIter = std::collections::hash_set::Iter<'a, String>;

    fn into_iter(self) -> Self::IntoIter {
        self.tickers.iter()
    }
}

impl TryFrom<&str> for Subscriptions {
    type Error = ClientError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let tickers = value
            .split(";")
            .map(|c| {
                if c.len() > CHARS_PER_TICKER as usize {
                    Err(ClientError::WrongSubscriptionString(value.to_owned()))
                } else {
                    Ok(c.to_owned())
                }
            })
            .collect::<Result<_, _>>()?;

        Ok(Subscriptions { tickers })
    }
}

impl TryFrom<String> for Subscriptions {
    type Error = ClientError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::try_from(value.as_str())
    }
}

impl TryFrom<&String> for Subscriptions {
    type Error = ClientError;

    fn try_from(value: &String) -> Result<Self, Self::Error> {
        Self::try_from(value.as_str())
    }
}

impl TryFrom<&[&str]> for Subscriptions {
    type Error = ClientError;

    fn try_from(value: &[&str]) -> Result<Self, Self::Error> {
        let tickers = value
            .iter()
            .map(|c| {
                if c.len() > CHARS_PER_TICKER as usize {
                    Err(ClientError::WrongSubscriptionString(c.to_string()))
                } else {
                    Ok(c.to_string())
                }
            })
            .collect::<Result<_, _>>()?;

        Ok(Subscriptions { tickers })
    }
}

impl std::fmt::Display for Subscriptions {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            self.tickers
                .clone()
                .into_iter()
                .collect::<Vec<String>>()
                .join(";")
        )
    }
}

#[allow(clippy::from_over_into)]
impl Into<Vec<String>> for Subscriptions {
    fn into(self) -> Vec<String> {
        self.tickers.into_iter().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subscription_creation() {
        // TC 1: Create from a list of &str
        let tickers = vec!["SAN", "REP", "IAG", "SAN"];
        let tickers_check = vec!["SAN", "REP", "IAG"];

        let test = Subscriptions::try_from(tickers.as_slice())
            .expect("Failed to build a Subscritpions map");

        assert_eq!(
            tickers_check.len(),
            (&test)
                .into_iter()
                .filter(|e| tickers_check.contains(&e.as_str()))
                .collect::<Vec<&String>>()
                .len()
        );
    }

    #[test]
    fn subscription_add() {
        let tickers_check = vec!["SAN", "SAB", "ACX"];
        let mut test = Subscriptions::default();

        test.add_subscriptions(&["SAN"]);
        assert_eq!(
            &["SAN"],
            (&test).into_iter().collect::<Vec<&String>>().as_slice()
        );
        test.add_subscriptions(&["SAN"]);
        assert_eq!(
            &["SAN"],
            (&test).into_iter().collect::<Vec<&String>>().as_slice()
        );
        test.add_subscriptions(&["SAB", "ACX"]);

        assert_eq!(
            (&test)
                .into_iter()
                .filter(|e| tickers_check.contains(&e.as_str()))
                .collect::<Vec<&String>>()
                .len(),
            tickers_check.len()
        );
    }

    #[test]
    fn subscription_remove() {
        let tickers = vec!["SAN", "REP", "IAG"];

        let mut test = Subscriptions::try_from(tickers.as_slice())
            .expect("Failed to build a Subscritpions map");

        test.remove_subscriptions(&["SAN"]);
        test.remove_subscriptions(&["SAN", "IAG"]);
        assert_eq!(
            &["REP"],
            (&test).into_iter().collect::<Vec<&String>>().as_slice()
        );
    }

    #[test]
    fn to_string() {
        let tickers = vec!["SAN", "REP"];

        let test = Subscriptions::try_from(tickers.as_slice())
            .expect("Failed to build a Subscritpions map");

        // A `HashSet` does not ensure any order of the elements, so we have 2 elements, we need to test two
        // possible strings.
        assert!(
            (test.to_string() == "SAN;REP".to_owned())
                || (test.to_string() == "REP;SAN".to_owned())
        );
    }

    #[test]
    fn is_subscribed() {
        let tickers = vec!["SAN", "REP", "IAG"];

        let test = Subscriptions::try_from(tickers.as_slice())
            .expect("Failed to build a Subscritpions map");

        assert!(test.is_subscribed(tickers.as_slice()));
    }
}
