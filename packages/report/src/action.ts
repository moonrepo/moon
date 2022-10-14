import { Action, ActionStatus } from '@moonrepo/types';
import { getDurationInMillis } from './time';

export function getIconForStatus(status: ActionStatus): string {
	// Use exhaustive checks!
	// eslint-disable-next-line default-case
	switch (status) {
		case 'cached':
			return '🟪';
		// case 'cached-remote':
		// 	return '🟦';
		case 'failed':
		case 'failed-and-abort':
			return '🟥';
		case 'invalid':
			return '🟨';
		case 'passed':
			return '🟩';
		case 'running':
		case 'skipped':
			return '⬛️';
	}

	return '⬜️';
}

export function hasFailed(status: ActionStatus): boolean {
	return status === 'failed' || status === 'failed-and-abort';
}

export function hasPassed(status: ActionStatus): boolean {
	return status === 'passed' || status === 'cached';
}

export function isFlaky(action: Action): boolean {
	if (action.flaky) {
		return true;
	}

	// The flaky field above didn't always exist!
	if (!action.attempts || action.attempts.length === 0) {
		return false;
	}

	return hasPassed(action.status) && action.attempts.some((attempt) => hasFailed(attempt.status));
}

export function isSlow(action: Action, slowThreshold: number): boolean {
	if (!action.duration) {
		return false;
	}

	const millis = getDurationInMillis(action.duration);
	const threshold = slowThreshold * 1000; // In seconds

	return millis > threshold;
}
