#! /bin/bash

# Create dummy push with current git HEAD branch
# ./<this>.sh | mysql [-u <user> -p] <dbname>

branch=`git rev-parse --symbolic-full-name HEAD`
hash=`git rev-parse HEAD`

echo INSERT INTO push_log
echo	'(ref, hash_before, hash_after, compare, repo_fname, head_msg)'
echo	"VALUES ('${branch}', '', '', 'https://', 'yappy/DollsKit', '1st push');"

echo INSERT INTO push_log
echo	'(ref, hash_before, hash_after, compare, repo_fname, head_msg)'
echo	"VALUES ('${branch}', '', '', 'https://', 'yappy/DollsKit', 'multiple\n\nline\ncomment');"

echo INSERT INTO push_log
echo	'(ref, hash_before, hash_after, compare, repo_fname, head_msg)'
echo	"VALUES ('${branch}', '', '', 'https://', 'yappy/DollsKit', '寿司ビール問題🍣🍺');"

echo INSERT INTO push_log
echo	'(ref, hash_before, hash_after, compare, repo_fname, head_msg)'
echo	"VALUES ('${branch}', '', '', 'https://', 'yappy/DollsKit', 'build test\n\n@update on');"

echo INSERT INTO push_log
echo	'(ref, hash_before, hash_after, compare, repo_fname, head_msg)'
echo	"VALUES ('${branch}', '', '', 'https://', 'yappy/DollsKit', 'build test\n\n@update off');"
