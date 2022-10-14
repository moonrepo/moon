import { Duration } from '@moonrepo/types';

export function getDurationInMillis(duration: Duration): number {
	return duration.secs * 1000 + duration.nanos / 1_000_000;
}

export function formatTime(mins: number, secs: number, millis: number): string {
	if (mins === 0 && secs === 0 && millis === 0) {
		return '0s';
	}

	const format = (val: number) => {
		let v = val.toFixed(1);

		if (v.endsWith('.0')) {
			v = v.slice(0, -2);
		}

		return v;
	};

	// When minutes, only show mins + secs
	if (mins > 0) {
		let value = `${mins}m`;

		if (secs > 0) {
			value += ` ${secs}s`;
		}

		return value;
	}

	// When seconds, only show secs + first milli digit
	if (secs > 0) {
		return `${format((secs * 1000 + millis) / 1000)}s`;
	}

	// When millis, show as is
	if (millis > 0) {
		return `${format(millis)}ms`;
	}

	// How did we get here?
	return '0s';
}

export function formatDuration(duration: Duration | null): string {
	if (!duration) {
		return '--';
	}

	if (duration.secs === 0 && duration.nanos === 0) {
		return '0s';
	}

	let mins = 0;
	let { secs } = duration;
	const millis = duration.nanos / 1_000_000;

	while (secs >= 60) {
		mins += 1;
		secs -= 60;
	}

	return formatTime(mins, secs, millis);
}
