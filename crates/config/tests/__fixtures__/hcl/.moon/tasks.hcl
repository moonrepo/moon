
locals {
	someCondition = true
	sharedInputs = ["src/**/*"]
}

fileGroups {
	sources = [
		"src/**/*"
	]
	tests = [
		"*.test.ts",
		"*.test.tsx",
	]
}

implicitDeps = [
	"project:task-a",
	{
		target = "project:task-b"
		optional = true
	},
	"project:task-c",
	{
		target = "project:task-d"
		args = ["--foo", "--bar"]
		env = {
			KEY = "value"
		}
	}
]

implicitInputs = [
	"$ENV",
	"$ENV_*",
	"file.txt",
	"file.*",
	"/file.txt",
	"/file.*",
]

taskOptions {
	affectedFiles = {
		pass = "args"
		passInputsWhenNoMatch = true
	}
	allowFailure = true
	cache = false
	envFile = ".env"
	interactive = false
	internal = true
	mergeArgs = "append"
	mergeDeps = "prepend"
	mergeEnv = "replace"
	mergeInputs = "preserve"
	mergeOutputs = null
	mutex = "lock"
	os = ["linux", "macos"]
	outputStyle = "stream"
	persistent = true
	retryCount = 3
	runDepsInParallel = false
	runInCI = true
	runFromWorkspaceRoot = false
	shell = false
	timeout = 60
	unixShell = "zsh"
	windowsShell = "pwsh"
}

tasks "example" {
	options {
		cache = local.someCondition
		cacheLifetime = local.someCondition ? "1 hour" : null
	}
}

tasks "test" {
	inputs = concat(local.sharedInputs, ["tests/**/*"])
}

tasks "lint" {
	inputs = concat(["**/*.graphql"], local.sharedInputs)
}

tasks "build-linux" {
	command = "cargo"
	args = ["--target", "x86_64-unknown-linux-gnu", "--verbose"]
	options {
		os = "linux"
	}
}

tasks "build-macos" {
	command = "cargo"
	args = ["--target", "x86_64-apple-darwin", "--verbose"]
	options {
		os = "macos"
	}
}

tasks "build-windows" {
	command = "cargo"
	args = ["--target", "i686-pc-windows-msvc", "--verbose"]
	options {
		os = "windows"
	}
}
