import { formatDuration, formatTime } from '../src';

describe('formatTime()', () => {
	it('handles all zeros', () => {
		expect(formatTime(0, 0, 0)).toBe('0s');
	});

	it('handles minutes', () => {
		expect(formatTime(5, 0, 555)).toBe('5m');
	});

	it('handles minutes with seconds', () => {
		expect(formatTime(5, 12, 555)).toBe('5m 12s');
	});

	it('handles seconds', () => {
		expect(formatTime(0, 59, 0)).toBe('59s');
	});

	it('handles seconds with millis', () => {
		expect(formatTime(0, 23, 250)).toBe('23.3s');
		expect(formatTime(0, 59, 900)).toBe('59.9s');
	});

	it('handles millis', () => {
		expect(formatTime(0, 0, 125)).toBe('125ms');
		expect(formatTime(0, 0, 895)).toBe('895ms');
	});
});

describe('formatDuration()', () => {
	it('returns nothing for missing duration', () => {
		expect(formatDuration(null)).toBe('--');
	});

	it('handles zeros', () => {
		expect(formatDuration({ secs: 0, nanos: 0 })).toBe('0s');
	});

	it('handles millis', () => {
		expect(formatDuration({ secs: 0, nanos: 2500 * 1_000_000 })).toBe('2500ms');
	});

	it('handles seconds', () => {
		expect(formatDuration({ secs: 12, nanos: 0 })).toBe('12s');
	});

	it('handles seconds with millis', () => {
		expect(formatDuration({ secs: 15, nanos: 2500 * 1_000_000 })).toBe('17.5s');
	});

	it('handles minutes', () => {
		expect(formatDuration({ secs: 85, nanos: 32_500 * 1_000_000 })).toBe('1m 25s');
	});
});
