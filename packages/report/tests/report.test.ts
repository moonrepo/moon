import type { RunReport } from '@moonrepo/types';
import { prepareReportActions, sortReport } from '../src';

function mockReport(): RunReport {
	return {
		actions: [
			{
				allowFailure: false,
				attempts: null,
				createdAt: '2022-09-12T22:50:12.621680Z',
				duration: {
					secs: 0,
					nanos: 0,
				},
				error: null,
				flaky: false,
				label: 'RunTask(types:build)',
				node: {
					action: 'sync-workspace',
				},
				nodeIndex: 0,
				operations: [],
				status: 'cached',
				finishedAt: '2022-09-12T22:50:12.932311Z',
				startedAt: '2022-09-12T22:50:12.932311Z',
			},
			{
				allowFailure: false,
				attempts: null,
				createdAt: '2022-09-12T22:50:12.932177Z',
				duration: {
					secs: 1922,
					nanos: 380_231_540,
				},
				error: null,
				flaky: true,
				label: 'RunTask(runtime:typecheck)',
				node: {
					action: 'sync-workspace',
				},
				nodeIndex: 1,
				operations: [],
				status: 'passed',
				finishedAt: '2022-09-12T22:50:12.932311Z',
				startedAt: '2022-09-12T22:50:12.932311Z',
			},
			{
				allowFailure: false,
				attempts: null,
				createdAt: '2022-09-12T22:50:12.932228Z',
				duration: {
					secs: 64,
					nanos: 571_634_134,
				},
				error: null,
				flaky: false,
				label: 'RunTask(types:typecheck)',
				node: {
					action: 'sync-workspace',
				},
				nodeIndex: 2,
				operations: [],
				status: 'passed',
				finishedAt: '2022-09-12T22:50:12.932311Z',
				startedAt: '2022-09-12T22:50:12.932311Z',
			},
			{
				allowFailure: false,
				attempts: null,
				createdAt: '2022-09-12T22:50:12.932311Z',
				duration: {
					secs: 34,
					nanos: 431_185_666,
				},
				error: null,
				flaky: false,
				label: 'RunTask(website:typecheck)',
				node: {
					action: 'sync-workspace',
				},
				nodeIndex: 3,
				operations: [],
				status: 'passed',
				finishedAt: '2022-09-12T22:50:12.932311Z',
				startedAt: '2022-09-12T22:50:12.932311Z',
			},
		],
		context: {
			affectedOnly: false,
			initialTargets: [],
			passthroughArgs: [],
			primaryTargets: [],
			profile: null,
			targetStates: {},
			touchedFiles: [],
		},
		duration: {
			secs: 0,
			nanos: 371_006_844,
		},
		comparisonEstimate: {
			duration: {
				secs: 1,
				nanos: 361_827_012,
			},
			tasks: {},
			loss: null,
			gain: {
				secs: 0,
				nanos: 990_820_168,
			},
			percent: 0,
		},
	};
}

describe('sortReport()', () => {
	it('sorts by time asc', () => {
		const report = mockReport();
		sortReport(report, 'time', 'asc');

		expect(report.actions.map((a) => a.label)).toEqual([
			'RunTask(types:build)',
			'RunTask(website:typecheck)',
			'RunTask(types:typecheck)',
			'RunTask(runtime:typecheck)',
		]);
	});

	it('sorts by time desc', () => {
		const report = mockReport();
		sortReport(report, 'time', 'desc');

		expect(report.actions.map((a) => a.label)).toEqual([
			'RunTask(runtime:typecheck)',
			'RunTask(types:typecheck)',
			'RunTask(website:typecheck)',
			'RunTask(types:build)',
		]);
	});

	it('sorts by label asc', () => {
		const report = mockReport();
		sortReport(report, 'label', 'asc');

		expect(report.actions.map((a) => a.label)).toEqual([
			'RunTask(runtime:typecheck)',
			'RunTask(types:build)',
			'RunTask(types:typecheck)',
			'RunTask(website:typecheck)',
		]);
	});

	it('sorts by label desc', () => {
		const report = mockReport();
		sortReport(report, 'label', 'desc');

		expect(report.actions.map((a) => a.label)).toEqual([
			'RunTask(website:typecheck)',
			'RunTask(types:typecheck)',
			'RunTask(types:build)',
			'RunTask(runtime:typecheck)',
		]);
	});
});

describe('prepareReportActions()', () => {
	it('returns actions prepared', () => {
		expect(prepareReportActions(mockReport(), 60)).toEqual([
			{
				comments: [],
				duration: {
					nanos: 0,
					secs: 0,
				},
				icon: '🟪',
				label: 'RunTask(types:build)',
				status: 'cached',
				time: '0s',
			},
			{
				comments: ['**FLAKY**', '**SLOW**'],
				duration: {
					nanos: 380_231_540,
					secs: 1922,
				},
				icon: '🟩',
				label: 'RunTask(runtime:typecheck)',
				status: 'passed',
				time: '32m 2s',
			},
			{
				comments: ['**SLOW**'],
				duration: {
					nanos: 571_634_134,
					secs: 64,
				},
				icon: '🟩',
				label: 'RunTask(types:typecheck)',
				status: 'passed',
				time: '1m 4s',
			},
			{
				comments: [],
				duration: {
					nanos: 431_185_666,
					secs: 34,
				},
				icon: '🟩',
				label: 'RunTask(website:typecheck)',
				status: 'passed',
				time: '34.4s',
			},
		]);
	});
});
