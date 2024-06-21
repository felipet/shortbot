-- Initial DB schema for the ShortBot application

CREATE TABLE Stock (
    ticker CHAR(4),
    market CHAR(4),
    full_name VARCHAR(80),
    name VARCHAR(80) NOT NULL,
    isin CHAR(12) NOT NULL,
    extra_id CHAR(11),
    PRIMARY KEY (ticker, market)
);

CREATE TABLE ShortPosition (
    id INT PRIMARY KEY AUTO_INCREMENT,
    notice_date date NOT NULL,
    owner VARCHAR(80) NOT NULL,
    weight FLOAT NOT NULL,
    ticker CHAR(4),
    market CHAR(4),
    FOREIGN KEY (ticker, market)
        REFERENCES Stock (ticker, market)
        ON UPDATE RESTRICT
        ON DELETE CASCADE
);

