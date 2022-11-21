import { ActionStatus, Duration, RunReport } from '@moonrepo/types';
import { getIconForStatus, isFlaky, isSlow } from './action';
import { formatDuration, getDurationInMillis } from './time';

export function sortReport(report: RunReport, sortBy: 'label' | 'time', sortDir: 'asc' | 'desc') {
	const isAsc = sortDir === 'asc';

	report.actions.sort((a, d) => {
		switch (sortBy) {
			case 'time': {
				const am = getDurationInMillis(a.duration ?? { nanos: 0, secs: 0 });
				const dm = getDurationInMillis(d.duration ?? { nanos: 0, secs: 0 });

				return isAsc ? am - dm : dm - am;
			}

			case 'label': {
				const al = a.label ?? '';
				const dl = d.label ?? '';

				return isAsc ? al.localeCompare(dl) : dl.localeCompare(al);
			}

			default:
				throw new Error(`Unknown sort by "${sortBy}".`);
		}
	});
}

export interface PreparedAction {
	comments: string[];
	duration: Duration | null;
	icon: string;
	label: string;
	status: ActionStatus;
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
			status: action.status,
			time: formatDuration(action.duration),
		};
	});
}
