# Changelog

## [0.1.0] - 2024-06-13

### Added

- Multilingual support for all the endpoints: Spanish and English supported.
- Custom implementation of the [finance_ibex](https://crates.io/crates/finance_ibex) lib.
- Feature to check alive short positions against stocks of the Ibex35.

## [0.2.0] - 2025-02-17

### Added

- Add cached results thanks to the integration with the *finance data harvest* module.
- Replaced local DB with the listing of Ibex35's companies by a listing from the DB.

## [0.3.0] - 2025-03-31

- Application delivery based on Podman images.
- Application CD thanks to the systemd feature *auto-update*.
- Bot receives updates using a webhook rather than long polling.
- Integrated `axum` Router to expose extra webhooks and connect other services to Shortbot.
- CI runs only after source code changes.

## [0.4.0] - 2025-07-4

### Added

- Add a new module to handle user's metadata.
- Add a new DB backend (Valkey) to store user's metadata.

### Changed

- Teloxide updated to v0.16.0
- Improved the tracing module to focus on logs from the `shortbot` module.

## [0.5.0] - 2025-07-10

### Fixed

- Fixed bug for pushing container images with multiple tags.

### Changed

- The User's metadata DB schema has been refactored to improve legibility and ease sharing a Valkey server with other
  applications.

## [0.6.0] - 2025-07-15

### Added

- New method added to `UserHandler` that returns a list of registered users of the bot.
- New handler added for the webhook endpoint of the main web server that delivers broadcast messages.
- Broadcast messages feature completely implemented.
- Teloxide updated to v0.17.0.

### Changed

- The user's schema in Valkey was extended to add a section for configuration parameters.

## [0.7.0] - 2025-07-28

### Added

- New menu to select stocks based on names rather than tickers.
- Keyboards module.
- New command /brief.
- New menu to handle subscriptions

### Changed

- Handlers read the user's language from the user's settings rather than from Telegram's API.
- The /help command accepts arguments to access different sections within the help menu.
- The format of the log messages has been improved to reduce redundant information. A new option to show pretty log messages for debugging has been added.
