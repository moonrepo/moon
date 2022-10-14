import { Duration, RunReport } from '@moonrepo/types';
import { getIconForStatus, isFlaky, isSlow } from './action';
import { formatDuration } from './time';

export function sortReport(report: RunReport, sortBy: string, sortDir: string) {
	const isAsc = sortDir === 'asc';

	report.actions.sort((a, d) => {
		switch (sortBy) {
			case 'time': {
				const at: Duration = a.duration ?? { nanos: 0, secs: 0 };
				const dt: Duration = d.duration ?? { nanos: 0, secs: 0 };
				const am = at.secs * 1000 + at.nanos / 1_000_000;
				const dm = dt.secs * 1000 + dt.nanos / 1_000_000;

				return isAsc ? am - dm : dm - am;
			}

			case 'label': {
				const al = a.label ?? '';
				const dl = d.label ?? '';

				return isAsc ? al.localeCompare(dl) : dl.localeCompare(al);
			}

			default: {
				return 0;
			}
		}
	});
}

export interface PreparedAction {
	comments: string[];
	duration: Duration | null;
	icon: string;
	label: string;
	time: string;
}

export function prepareReportActions(report: RunReport, slowThreshold: number): PreparedAction[] {
	return report.actions.map((action) => {
		const comments: string[] = [];

		if (isFlaky(action)) {
			comments.push('**FLAKY**');
		}

		if (action.attempts && action.attempts.length > 1) {
			comments.push(`${action.attempts.length} attempts`);
		}

		if (isSlow(action, slowThreshold)) {
			comments.push('**SLOW**');
		}

		return {
			comments,
			duration: action.duration,
			icon: getIconForStatus(action.status),
			label: action.label ?? '<unknown>',
			time: formatDuration(action.duration),
		};
	});
}
