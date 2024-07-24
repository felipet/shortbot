-- Trigger that maintans the Alive table updated after a new
-- short position was added.

CREATE TRIGGER update_alive_positions
AFTER INSERT
ON `ShortPosition` FOR EACH ROW
BEGIN
    DECLARE existing INT;
    DECLARE old_date DATE;
    SET existing = 0;

    SELECT a.id, sp.notice_date INTO existing, old_date
    FROM `ShortPosition` sp, shortbot.Alive a
    WHERE   sp.ticker = NEW.ticker
            AND sp.market = NEW.market
            AND sp.id = a.position
            AND sp.owner = NEW.owner;

    IF existing > 0 THEN
        IF NEW.notice_date > old_date THEN
            UPDATE `Alive` SET position = NEW.id WHERE id = existing;
        END IF;
    ELSE
        INSERT INTO `Alive` (`position`) VALUES(NEW.id);
    END IF;
END ;
