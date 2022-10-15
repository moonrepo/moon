import { RunReport } from '@moonrepo/types';
import { prepareReportActions, sortReport } from '../src';

function mockReport(): RunReport {
	return {
		actions: [
			{
				attempts: null,
				createdAt: '2022-09-12T22:50:12.621680Z',
				duration: {
					secs: 0,
					nanos: 0,
				},
				error: null,
				flaky: false,
				label: 'RunTarget(types:build)',
				nodeIndex: 5,
				status: 'cached',
			},
			{
				attempts: null,
				createdAt: '2022-09-12T22:50:12.932177Z',
				duration: {
					secs: 1922,
					nanos: 380_231_540,
				},
				error: null,
				flaky: true,
				label: 'RunTarget(runtime:typecheck)',
				nodeIndex: 4,
				status: 'passed',
			},
			{
				attempts: null,
				createdAt: '2022-09-12T22:50:12.932228Z',
				duration: {
					secs: 64,
					nanos: 571_634_134,
				},
				error: null,
				flaky: false,
				label: 'RunTarget(types:typecheck)',
				nodeIndex: 6,
				status: 'passed',
			},
			{
				attempts: null,
				createdAt: '2022-09-12T22:50:12.932311Z',
				duration: {
					secs: 34,
					nanos: 431_185_666,
				},
				error: null,
				flaky: false,
				label: 'RunTarget(website:typecheck)',
				nodeIndex: 8,
				status: 'passed',
			},
		],
		context: {
			passthroughArgs: [],
			primaryTargets: [],
			profile: null,
			touchedFiles: [],
		},
		duration: {
			secs: 0,
			nanos: 371_006_844,
		},
		estimatedSavings: {
			secs: 0,
			nanos: 990_820_168,
		},
		projectedDuration: {
			secs: 1,
			nanos: 361_827_012,
		},
	};
}

describe('sortReport()', () => {
	it('sorts by time asc', () => {
		const report = mockReport();
		sortReport(report, 'time', 'asc');

		expect(report.actions.map((a) => a.label)).toEqual([
			'RunTarget(types:build)',
			'RunTarget(website:typecheck)',
			'RunTarget(types:typecheck)',
			'RunTarget(runtime:typecheck)',
		]);
	});

	it('sorts by time desc', () => {
		const report = mockReport();
		sortReport(report, 'time', 'desc');

		expect(report.actions.map((a) => a.label)).toEqual([
			'RunTarget(runtime:typecheck)',
			'RunTarget(types:typecheck)',
			'RunTarget(website:typecheck)',
			'RunTarget(types:build)',
		]);
	});

	it('sorts by label asc', () => {
		const report = mockReport();
		sortReport(report, 'label', 'asc');

		expect(report.actions.map((a) => a.label)).toEqual([
			'RunTarget(runtime:typecheck)',
			'RunTarget(types:build)',
			'RunTarget(types:typecheck)',
			'RunTarget(website:typecheck)',
		]);
	});

	it('sorts by label desc', () => {
		const report = mockReport();
		sortReport(report, 'label', 'desc');

		expect(report.actions.map((a) => a.label)).toEqual([
			'RunTarget(website:typecheck)',
			'RunTarget(types:typecheck)',
			'RunTarget(types:build)',
			'RunTarget(runtime:typecheck)',
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
				label: 'RunTarget(types:build)',
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
				label: 'RunTarget(runtime:typecheck)',
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
				label: 'RunTarget(types:typecheck)',
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
				label: 'RunTarget(website:typecheck)',
				status: 'passed',
				time: '34.4s',
			},
		]);
	});
});
