dependsOn = [
	"a",
	{
		id = "b"
		scope = "build"
		source = "implicit"
	}
]

docker {
	file {
		buildTask = "build"
		image = "node:latest"
		startTask = "start"
	}
	scaffold {
		sourcesPhaseGlobs = ["*.js"]
	}
}

env {
	KEY = "value"
}

fileGroups {
	sources = ["src/**/*"]
	tests = ["/**/*.test.*"]
}

id = "custom-id"

language = "rust"

owners {
	customGroups {}
	defaultOwner = "owner"
	optional = true
	paths = ["dir/", "file.txt"]
	requiredApprovals = 5
}

project {
	title = "Name"
	description = "Does something"
	owner = "team"
	channel = "#team"
	bool = true
	string = "abc"
}

stack = "frontend"

tags = ["a", "b", "c"]

tasks {}

toolchains {
	deno {
		version = "1.2.3"
	}
	typescript {
		includeSharedTypes = true
	}
}

layer = "library"

workspace {
	inheritedTasks {
		exclude = ["build"]
		include = ["test"]
		rename {
			old = "new"
		}
	}
}
