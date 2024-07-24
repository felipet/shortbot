-- ----------------------------------------------
-- Initial DB schema for the Shortbot application
-- ----------------------------------------------

--
-- Table structure for table `Fund`
--
DROP TABLE IF EXISTS `Fund`;

CREATE TABLE `Fund` (
  `id` int(11) NOT NULL AUTO_INCREMENT,
  `name` varchar(80) NOT NULL,
  PRIMARY KEY (`id`),
  UNIQUE KEY `name` (`name`)
);

--
-- Table structure for table `Stock`
--
DROP TABLE IF EXISTS `Stock`;

CREATE TABLE `Stock` (
  `ticker` char(4) NOT NULL,
  `market` char(4) NOT NULL,
  `full_name` varchar(80) DEFAULT NULL,
  `name` varchar(80) NOT NULL,
  `isin` char(12) NOT NULL,
  `extra_id` char(10) DEFAULT NULL,
  PRIMARY KEY (`ticker`,`market`),
  UNIQUE KEY `isin` (`isin`),
  UNIQUE KEY `extra_id` (`extra_id`)
);

--
-- Table structure for table `ShortPosition`
--
DROP TABLE IF EXISTS `ShortPosition`;

CREATE TABLE `ShortPosition` (
  `id` int(11) NOT NULL AUTO_INCREMENT,
  `ticker` char(4) NOT NULL,
  `market` char(4) NOT NULL,
  `notice_date` date NOT NULL,
  `percentage` float DEFAULT NULL,
  `owner` int(11) NOT NULL,
  PRIMARY KEY (`id`),
  KEY `ShortPosition_Stock_FK` (`ticker`,`market`),
  KEY `ShortPosition_ibfk_1` (`owner`),
  CONSTRAINT `ShortPosition_Stock_FK` FOREIGN KEY (`ticker`, `market`) REFERENCES `Stock` (`ticker`, `market`) ON DELETE CASCADE,
  CONSTRAINT `ShortPosition_ibfk_1` FOREIGN KEY (`owner`) REFERENCES `Fund` (`id`) ON DELETE CASCADE ON UPDATE CASCADE
);

--
-- Table structure for table `Alive`
--
DROP TABLE IF EXISTS `Alive`;

CREATE TABLE `Alive` (
  `id` int(11) NOT NULL AUTO_INCREMENT,
  `position` int(11) NOT NULL,
  PRIMARY KEY (`id`),
  KEY `Alive_ShortPosition_FK` (`position`),
  CONSTRAINT `Alive_ShortPosition_FK` FOREIGN KEY (`position`) REFERENCES `ShortPosition` (`id`) ON DELETE CASCADE
);