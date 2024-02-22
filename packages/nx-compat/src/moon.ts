/* eslint-disable no-nested-ternary */

import path from 'node:path';
import { execa } from 'execa';
import type {
	ProjectGraph as NxProjectGraph,
	ProjectGraphProjectNode,
} from 'nx/src/config/project-graph';
import type { TaskGraph as NxTaskGraph } from 'nx/src/config/task-graph';
import type {
	ProjectConfiguration as NxProject,
	TargetConfiguration as NxTarget,
} from 'nx/src/config/workspace-json-project-json';
import { json } from '@boost/common';
import type { ActionGraph, Project, ProjectGraph, Task } from '@moonrepo/types';
import { env, loadAndCacheJson } from './helpers';

export async function loadActionGraph(root: string): Promise<ActionGraph> {
	return loadAndCacheJson('action-graph', root, async () => {
		const result = await execa('moon', ['action-graph', '--json', '--log', 'off'], { cwd: root });

		return json.parse(result.stdout);
	});
}

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

export function createNxTaskGraphFromMoonGraphs(
	actionGraph: ActionGraph,
	projectGraph: ProjectGraph,
): NxTaskGraph {
	const graph: NxTaskGraph = {
		dependencies: {},
		roots: [],
		tasks: {},
	};

	actionGraph.nodes.forEach((node) => {
		if (!node.label.startsWith('Run')) {
			return;
		}

		const target = node.label
			.replace('RunTarget(', '')
			.replace('RunTask(', '')
			.replace('RunInteractiveTask(', '')
			.replace('RunPersistentTask(', '')
			.replace(')', '');
		const [projectId, taskId] = target.split(':');
		const project = projectGraph.projects[projectId];

		if (!project) {
			return;
		}

		const task = project.tasks[taskId];

		if (!task) {
			return;
		}

		graph.tasks[String(node.id)] = {
			id: String(node.id),
			outputs: [...task.outputFiles, ...task.outputGlobs],
			overrides: {},
			projectRoot: project.root,
			target: { project: projectId, target: taskId },
		};
	});

	actionGraph.edges.forEach((edge) => {
		if (graph.tasks[String(edge.source)]) {
			(graph.dependencies[edge.source] ||= []).push(String(edge.target));
		}
	});

	return graph;
}
