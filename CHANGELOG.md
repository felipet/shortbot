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
