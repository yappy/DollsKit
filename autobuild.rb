# Each command will be executed with the same stdin/stdout/stderr
# of this script.
# This script will also print to stdout/stderr.

require 'date'

def Task(program, *args)
	puts "[#{DateTime.now}] Task start"
	puts (["% ", program] + args.map{|arg| "'#{arg}'" }).join(" ")
	if system(program, *args) then
		puts "[#{DateTime.now}] Task completed successfully"
		puts
	else
		puts "[#{DateTime.now}] Task failed"
		exit false
	end
end

usage = "Usage: ruby <this>.rb <branch_name>"
branch_name = ARGV[0] or abort(usage)
# remove "/refs/heads/"
branch_name = branch_name.gsub(/^refs\/heads\//, "")


puts "===== Auto build (branch=#{branch_name}) start ====="
puts

Task("git", "fetch", "--all")
Task("git", "checkout", branch_name)
Task("git", "merge", "--ff-only", "origin/#{branch_name}")

Task("make")
