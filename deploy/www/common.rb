module Const
	TEST_PIC = "pics/test.jpg"
	PIC_DIRS = "pics/*".freeze
	PIC_PATTERN = /pics\/(\d+)/.freeze
	TH_FILES = "pics/%s/*_th.jpg".freeze
	TH_PATTERN = /pics\/.*\/(\d+_\d+)_th.jpg/.freeze
	PIC_FILE = "pics/%s/%s.jpg".freeze
	UP_FILE = "twque/%s.jpg".freeze
end
