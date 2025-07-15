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

//! Keyboards module
//!
//! # Description
//!
//! This module includes all the keyboards that are used within the handlers of the bot.

use std::collections::HashSet;

use finance_api::Company;
use finance_ibex::IbexCompany;
use teloxide::types::{InlineKeyboardButton, InlineKeyboardMarkup};

/// How many buttons to show per row.
const BUTTONS_PER_ROW: usize = 5;
/// How many buttons to show per row when using full names.
const NAMES_PER_ROW: usize = 2;

/// Inline keyboard that lists tickers in a grid.
pub fn tickers_grid_keyboard(ibex_companies: &[IbexCompany]) -> InlineKeyboardMarkup {
    // We only need the ticker field, so let's build a list of tickers.
    let market: Vec<&str> = ibex_companies.iter().map(|e| e.ticker()).collect();
    let stock_len = market.len();

    // Populate the first row
    let mut keyboard_markup = InlineKeyboardMarkup::new([vec![InlineKeyboardButton::callback::<
        &str,
        &str,
    >(market[0], market[0])]]);

    for company in market.iter().take(BUTTONS_PER_ROW).skip(1) {
        keyboard_markup = keyboard_markup.append_to_row(
            0,
            InlineKeyboardButton::callback::<&str, &str>(company, company),
        );
    }

    // Populate rows by chunks of `BUTTONS_PER_ROW` buttons
    for i in 1..(stock_len / BUTTONS_PER_ROW) {
        for j in 0..BUTTONS_PER_ROW {
            keyboard_markup = keyboard_markup.append_to_row(
                i,
                InlineKeyboardButton::callback::<&str, &str>(
                    market[j + i * BUTTONS_PER_ROW],
                    market[j + i * BUTTONS_PER_ROW],
                ),
            );
        }
    }

    // Finally, add the remainder in case the number of items is not divisible by `BUTTONS_PER_ROW`
    if stock_len % BUTTONS_PER_ROW != 0 {
        let mut i = stock_len - BUTTONS_PER_ROW;
        while i < stock_len {
            keyboard_markup = keyboard_markup.append_to_row(
                stock_len / BUTTONS_PER_ROW + 1,
                InlineKeyboardButton::callback::<&str, &str>(market[i], market[i]),
            );

            i += 1;
        }
    }

    keyboard_markup
}

pub fn companies_keyboard(
    ibex_companies: &[IbexCompany],
    filter: Option<&str>,
) -> InlineKeyboardMarkup {
    // Build a keyboard of capital letters.
    if filter.is_none() {
        let mut keyboard_markup = InlineKeyboardMarkup::default();

        for c in starting_char_grid(ibex_companies).chunks(BUTTONS_PER_ROW) {
            keyboard_markup =
                keyboard_markup.append_row(c.iter().map(|c| InlineKeyboardButton::callback(c, c)));
        }

        keyboard_markup
    // Build a keyboard of company names
    } else {
        let mut keyboard_markup = InlineKeyboardMarkup::default();
        let filter = filter.unwrap();

        // We push companies to the new keyboard whose first letter is equal to `filter` or, if the company's name
        // includes a white space, whose first letter of the last word of the name is equal to `filter`.
        // Rather tricky, but it would allow addressing Banco Sabadell by either `B` or `S`.
        for company in ibex_companies
            .iter()
            .filter(|c| {
                &c.name()[..1] == filter
                    || &c
                        .name()
                        .split(" ")
                        .collect::<Vec<_>>()
                        .iter()
                        .last()
                        .unwrap()[..1]
                        == filter
            })
            .collect::<Vec<_>>()
            .chunks(NAMES_PER_ROW)
        {
            keyboard_markup = keyboard_markup.append_row(
                company
                    .iter()
                    .map(|c| InlineKeyboardButton::callback(c.name(), c.name())),
            );
        }

        keyboard_markup
    }
}

/// Make a list with the first char of the Ibex35 companies.
fn starting_char_grid(ibex_companies: &[IbexCompany]) -> Vec<String> {
    let mut chars_set = HashSet::new();

    for item in ibex_companies {
        if let Some(first_char) = item.name().chars().next() {
            chars_set.insert(first_char.to_string());
        }
        // Allow pushing characters for composed names like Banco Sabadell (either B or S).
        if let Some(first_char) = item.name().split(" ").collect::<Vec<_>>().iter().last() {
            chars_set.insert(first_char.to_string()[..1].to_owned());
        }
    }

    let mut result: Vec<_> = chars_set.into_iter().collect();
    result.sort();

    result
}
