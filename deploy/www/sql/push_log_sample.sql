# Please specify your database name

INSERT INTO push_log
	(ref, hash_before, hash_after, compare, repo_fname, head_msg)
	VALUES ("refs/heads/master", "", "", "https://github.com/", "yappy/DollsKit", "1st push");

INSERT INTO push_log
	(ref, hash_before, hash_after, compare, repo_fname, head_msg)
	VALUES ("refs/heads/test", "", "", "https://github.com/", "yappy/DollsKit", "multiple\n\nline\ncomment");

# The last push is refs/heads/master, so should pull and build it
INSERT INTO push_log
	(ref, hash_before, hash_after, compare, repo_fname, head_msg)
	VALUES ("refs/heads/master", "", "", "https://github.com/", "yappy/DollsKit", "寿司ビール問題🍣🍺");
