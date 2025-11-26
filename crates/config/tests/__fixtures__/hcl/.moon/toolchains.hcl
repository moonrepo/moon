typescript {
	createMissingConfig = false
	includeProjectReferenceSources = true
	includeSharedTypes = true
	projectConfigFileName = "tsconfig.app.json"
	rootConfigFileName = "tsconfig.root.json"
	rootOptionsConfigFileName = "tsconfig.opts.json"
	routeOutDirToCache = true
	syncProjectReferences = false
	syncProjectReferencesToPaths = true
}

node {
	plugin = "file://node.wasm"
	version = "20"
}

proto {
	version = "1.2.3"
}
