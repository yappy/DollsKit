# Please specify your database name

DROP TABLE IF EXISTS push_log;

CREATE TABLE push_log(
	id			INT AUTO_INCREMENT NOT NULL,
	created_at	TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
	ref			VARCHAR(256) NOT NULL,
	hash_before	CHAR(40) NOT NULL,
	hash_after	CHAR(40) NOT NULL,
	compare		VARCHAR(256) NOT NULL,
	repo_fname	VARCHAR(256) NOT NULL,
	head_msg	VARCHAR(256) NOT NULL,
	PRIMARY KEY(id)
);
