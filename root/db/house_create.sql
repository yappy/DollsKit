CREATE TABLE system_logs (
  id SERIAL NOT NULL PRIMARY KEY,
  level_id INT NOT NULL,
  dt DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP ON UPDATE CURRENT_TIMESTAMP,
  short_desc VARCHAR(255) NOT NULL,
);

/*
 +----+----------+----------+
 | id | name     | severity |
 +----+----------+----------+
 |  1 | critical |        0 |
 |  2 | error    |        1 |
 |  3 | warning  |        2 |
 |  4 | info     |        3 |
 |  5 | debug    |        4 |
 +----+----------+----------+
 */
CREATE TABLE log_levels (
  id SERIAL NOT NULL PRIMARY KEY,
  name VARCHAR(16) NOT NULL,
  severity INT UNSIGNED NOT NULL
);

INSERT INTO
  log_levels (name, severity)
VALUES
  ('critical', 0);

INSERT INTO
  log_levels (name, severity)
VALUES
  ('error', 1);

INSERT INTO
  log_levels (name, severity)
VALUES
  ('warning', 2);

INSERT INTO
  log_levels (name, severity)
VALUES
  ('info', 3);

INSERT INTO
  log_levels (name, severity)
VALUES
  ('debug', 4);
