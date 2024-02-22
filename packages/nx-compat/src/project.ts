/* eslint-disable no-nested-ternary */

import path from 'node:path';
import { execa } from 'execa';
import type {
	ProjectGraph as NxProjectGraph,
	ProjectGraphProjectNode,
} from 'nx/src/config/project-graph';
import type {
	ProjectConfiguration as NxProject,
	TargetConfiguration as NxTarget,
} from 'nx/src/config/workspace-json-project-json';
import { json } from '@boost/common';
import type { Project, ProjectGraph, Task } from '@moonrepo/types';
import { env, loadAndCacheJson } from './helpers';

export async function loadProjectSnapshot(): Promise<Project> {
	return json.load(env('MOON_PROJECT_SNAPSHOT'));
}

export async function loadProjectGraph(root: string): Promise<ProjectGraph> {
	return loadAndCacheJson('project-graph', root, async () => {
		const result = await execa('moon', ['project-graph', '--json', '--log', 'off'], { cwd: root });

		return json.parse(result.stdout);
	});
}

export function createNxTargetFromMoonTask(task: Task): NxTarget {
	const inputs: NxTarget['inputs'] = [];

	task.inputFiles.forEach((input) => {
		inputs.push({ input });
	});

	task.inputGlobs.forEach((input) => {
		inputs.push({ fileset: input });
	});

	task.inputVars.forEach((input) => {
		inputs.push({ env: input });
	});

	return {
		cache: task.options.cache,
		command: task.command,
		dependsOn: task.deps.map((dep) => dep.target),
		inputs,
		options: {},
		outputs: [...task.outputFiles, ...task.outputGlobs],
	};
}

export function createNxProjectFromMoonProject(project: Project): NxProject {
	const namedInputs: NxProject['namedInputs'] = {};
	const targets: NxProject['targets'] = {};

	Object.entries(project.fileGroups).forEach(([name, group]) => {
		namedInputs[name] = [...group.files, ...group.globs];
	});

	Object.entries(project.tasks).forEach(([name, task]) => {
		targets[name] = createNxTargetFromMoonTask(task);
	});

	return {
		implicitDependencies: project.dependencies
			.filter((dep) => dep.source === 'explicit')
			.map((dep) => dep.id),
		name: project.id,
		namedInputs,
		projectType: project.type === 'application' ? 'application' : 'library',
		root: project.source,
		sourceRoot: path.join(project.source, 'src'),
		tags: project.config.tags,
		targets,
	};
}

export function createNxProjectGraphFromMoonProjectGraph(
	projectGraph: ProjectGraph,
): NxProjectGraph {
	const graph: NxProjectGraph = {
		dependencies: {},
		nodes: {},
	};

	Object.values(projectGraph.projects).forEach((project) => {
		const node: ProjectGraphProjectNode = {
			data: createNxProjectFromMoonProject(project),
			name: project.id,
			type: project.type === 'application' ? 'app' : project.type === 'automation' ? 'e2e' : 'lib',
		};

		graph.nodes[project.id] = node;

		if (project.alias) {
			graph.nodes[project.alias] = node;
		}
	});

	projectGraph.graph.edges.forEach((edge) => {
		const source = projectGraph.graph.nodes[edge[0]];
		const target = projectGraph.graph.nodes[edge[1]];

		if (source && target) {
			(graph.dependencies[source.id] ||= []).push({
				source: source.id,
				target: target.id,
				type: 'static',
			});
		}
	});

	return graph;
}

export function createNxTargetFromSnapshot(
	snapshot: Project,
	executorTarget: string,
	taskName: string,
) {
	const target = createNxTargetFromMoonTask(snapshot.tasks[taskName]);
	target.executor = executorTarget;

	return target;
}
