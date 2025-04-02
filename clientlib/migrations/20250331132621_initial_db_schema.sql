-- Initial DB Schema for a client of the Bot.
CREATE TABLE `BotClient` (
  `id` bigint PRIMARY KEY,
  `registered` bool NOT NULL,
  `access` ENUM ('free', 'limited', 'unlimited', 'admin') NOT NULL,
  `subscriptions` varchar(174),
  `created_at` timestamp DEFAULT CURRENT_TIMESTAMP(),
  `last_access` timestamp
);
