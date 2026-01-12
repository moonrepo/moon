description = "I do something"

command = "cmd --arg"

args = ["-c", "-b", "arg"]

deps = [
	"proj:task",
	{
		target = "^:build"
		optional = true
	},
	{
		target = "~:build"
		args = ["--minify"]
		env = {
			DEBUG = "1"
		}
	}
]

env {
	ENV = "development"
}

inputs = [
	"$ENV",
	"$ENV_*",
	"file.txt",
	"file.*",
	"/file.txt",
	"/file.*",
	"@dirs(name)",
]

outputs = [
	"$workspaceRoot",
	"file.txt",
	"file.*",
	"/file.txt",
	"/file.*",
]

options {
	cache = false
	retryCount = 3
}

preset = "server"

type = "build"
