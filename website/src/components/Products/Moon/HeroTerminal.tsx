/* eslint-disable sort-keys */

import React, { useCallback, useEffect, useRef, useState } from 'react';

const COMMAND_TO_RUN = '$ moon run :build';

function random(min: number, max: number) {
	const minimum = Math.ceil(min - 50);

	return Math.floor(Math.random() * (Math.floor(max - 100) - minimum + 1)) + minimum;
}

function num(value: number) {
	return value > 1000 ? `${value / 1000}s` : `${value}ms`;
}

const TARGETS_CHAIN: [string, number][][] = [
	[['theme-tokens:build', random(450, 800)]],
	[
		['design-system:build', random(250, 1000)],
		['accessibility:build', random(350, 800)],
		['test-utils:build', random(250, 700)],
	],
	[
		['data-layer:build', random(500, 1000)],
		['server:build', random(1400, 2100)],
	],
	[['client:build', random(1300, 1900)]],
	[
		['web:build', random(850, 2150)],
		['mobile:build', random(750, 1550)],
	],
];

interface LineProps {
	type: 'finish' | 'log' | 'start';
	message: string;
	time?: number;
}

function Line({ type, time, message }: LineProps) {
	return (
		<li className="list-none">
			{type === 'start' && (
				<>
					<span className="text-gray-900">▪</span>
					<span className="text-gray-800">▪</span>
					<span className="text-gray-700">▪</span>
					<span className="text-gray-600">▪</span>{' '}
				</>
			)}
			{type === 'finish' && (
				<>
					<span className="text-purple-700">▪</span>
					<span className="text-purple-600">▪</span>
					<span className="text-purple-500">▪</span>
					<span className="text-purple-400">▪</span>{' '}
				</>
			)}
			{message}
			{time && <span className="text-sm text-gray-700"> ({num(time)})</span>}
		</li>
	);
}

export default function HeroTerminal() {
	const terminal = useRef<HTMLUListElement>(null);
	const [typingIndex, setTypingIndex] = useState(0);
	const [targetIndex, setTargetIndex] = useState(-1);
	const [lines, setLines] = useState<LineProps[]>([]);
	const [startTime, setStartTime] = useState(0);
	const [stopTime, setStopTime] = useState(0);
	const isMounted = useRef(false);

	useEffect(() => {
		isMounted.current = true;

		return () => {
			isMounted.current = false;
		};
	}, []);

	const runTimeout = useCallback((handler: () => void, delay: number) => {
		if (!isMounted.current) {
			return;
		}

		setTimeout(() => {
			if (isMounted.current) {
				handler();
			}
		}, delay);
	}, []);

	// Emulate the command being typed into the box
	useEffect(() => {
		if (typingIndex < COMMAND_TO_RUN.length) {
			runTimeout(() => {
				setTypingIndex((prev) => prev + 1);
			}, 125);
		} else {
			setStartTime(Date.now());
			setTargetIndex(0);
		}
	}, [typingIndex]);

	// Emulate a bunch of targets running
	useEffect(() => {
		if (targetIndex < 0) {
			return;
		}

		if (targetIndex >= TARGETS_CHAIN.length) {
			setStopTime(Date.now());

			runTimeout(() => {
				setTypingIndex(0);
				setTargetIndex(-1);
				setLines([]);
				setStartTime(0);
				setStopTime(0);
			}, 10_000);

			return;
		}

		const targets = TARGETS_CHAIN[targetIndex];
		const longestDuration = Math.max(...targets.map((line) => line[1]));

		targets.forEach(([target, duration]) => {
			setLines((prev) => [...prev, { type: 'start', message: target }]);

			runTimeout(() => {
				setLines((prev) => [...prev, { type: 'finish', message: target, time: duration }]);
			}, duration);
		});

		// Set a delay to start the next targets
		runTimeout(() => {
			setTargetIndex((prev) => prev + 1);
		}, longestDuration + 100);
	}, [targetIndex]);

	// Scroll to bottom when lines change
	useEffect(() => {
		if (terminal.current) {
			terminal.current.scrollTop = terminal.current.scrollHeight * 2;
		}
	}, [lines, stopTime]);

	return (
		<ul
			className="flex flex-col w-full p-2 m-0 overflow-auto font-mono text-sm text-gray-200 border border-solid rounded-lg bg-slate-900 border-slate-500"
			style={{ height: 230 }}
			ref={terminal}
		>
			<li className="list-none">
				<strong>{COMMAND_TO_RUN.slice(0, typingIndex)}</strong>
			</li>

			{lines.map((line) => (
				<Line key={line.type + line.message} {...line} />
			))}

			{stopTime > 0 && (
				<>
					<li className="pt-2 list-none">
						<strong className="text-gray-600">Tasks</strong>:{' '}
						<span className="text-green-500">{lines.length / 2} completed</span>
					</li>

					<li className="list-none">
						<strong className="text-gray-600">
							<span className="invisible">T</span>Time
						</strong>
						: {num(stopTime - startTime)}
					</li>
				</>
			)}
		</ul>
	);
}
