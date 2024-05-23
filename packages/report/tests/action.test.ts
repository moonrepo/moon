import type { Action } from '@moonrepo/types';
import { isFlaky, isSlow } from '../src';

const action: Action = {
	allowFailure: false,
	attempts: null,
	createdAt: '2022-09-12T22:50:12.932311Z',
	duration: {
		secs: 34,
		nanos: 431_185_666,
	},
	error: null,
	flaky: false,
	label: 'RunTask(app:build)',
	node: {
		action: 'sync-workspace',
	},
	nodeIndex: 0,
	operations: [],
	status: 'passed',
	finishedAt: '2022-09-12T22:50:12.932311Z',
	startedAt: '2022-09-12T22:50:12.932311Z',
};

describe('isFlaky()', () => {
	it('returns false by default', () => {
		expect(isFlaky({ ...action })).toBe(false);
	});

	it('returns true if flaky field is true', () => {
		expect(isFlaky({ ...action, flaky: true })).toBe(true);
	});
});

describe('isSlow()', () => {
	it('returns false for no duration', () => {
		expect(isSlow({ ...action, duration: null }, 1)).toBe(false);
	});

	it('returns false if below threshold', () => {
		expect(isSlow({ ...action, duration: { secs: 1, nanos: 0 } }, 2)).toBe(false);
	});

	it('returns true if above threshold', () => {
		expect(isSlow({ ...action, duration: { secs: 3, nanos: 0 } }, 2)).toBe(true);
	});
});
