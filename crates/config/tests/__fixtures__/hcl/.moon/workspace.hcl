codeowners {
	globalPaths = {
		"*" = ["@admins"]
	}
	orderBy = "project-id"
	requiredApprovals = 1
	sync = true
}

constraints {
	enforceLayerRelationships = false
	tagRelationships = {
		a = ["b", "c"]
	}
}

docker {
	prune {
		deleteVendorDirectories = false
		installToolchainDependencies = false
	}
	scaffold {
		configsPhaseGlobs = ["*.js"]
	}
}

generator {
	templates = [
		"/shared-templates",
		"./templates",
	]
}

hasher {
	ignorePatterns = ["*.map"]
	ignoreMissingPatterns = [".env"]
	optimization = "performance"
	walkStrategy = "vcs"
	warnOnMissingInputs = true
}

notifier {
	webhookUrl = "http://localhost"
}

projects {
	globs = ["apps/*", "packages/*"]
	sources = {
		root = "."
	}
}

pipeline {
	autoCleanCache = false
	cacheLifetime = "1 day"
	inheritColorsForPipedTasks = false
	logRunningCommand = true
}

telemetry = false

vcs {
	defaultBranch = "main"
	hooks = {
		pre-commit = ["moon check --all --affected", "moon run :pre-commit"]
	}
	client = "git"
	provider = "gitlab"
	remoteCandidates = [
		"main",
		"origin/main",
	]
	sync = true
}

versionConstraint = ">=1.2.3"
