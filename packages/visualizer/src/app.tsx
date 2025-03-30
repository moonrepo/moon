import './app.css';
import { useState } from 'preact/hooks';
import { Graph } from './components/Graph';

const SUPPORTED_LAYOUTS = ['dagre', 'klay', 'breadthfirst', 'grid'];

function getLayoutFromQuery(): string {
	let layout = new URLSearchParams(window.location.search).get('layout');

	if (!layout || !SUPPORTED_LAYOUTS.includes(layout)) {
		layout = 'dagre';
	}

	return layout;
}

function setLayoutIntoQuery(layout: string) {
	const query = new URLSearchParams(window.location.search);
	query.set('layout', layout);

	// Doesn't reload the page
	window.history.pushState(null, '', `${window.location.pathname}?${query}`);
}

export function App() {
	const [layout, setLayout] = useState(getLayoutFromQuery());

	function handleChange(event: Event) {
		const target = event.target as HTMLSelectElement;
		const newLayout = target.value;

		setLayout(newLayout);
		setLayoutIntoQuery(newLayout);
	}

	return (
		<main>
			<div className="p-4 flex items-center float-right">
				<span className="inline-block mr-1">Layout:</span>

				<select
					className="border border-slate-400 rounded bg-slate-600 text-slate-50 p-1"
					value={layout}
					onChange={handleChange}
				>
					{SUPPORTED_LAYOUTS.map((value) => (
						<option key={value} value={value}>
							{value}
						</option>
					))}
				</select>
			</div>

			<h2 className="m-0 p-4 text-3xl font-extrabold sm:text-4xl">{window.PAGE_TITLE}</h2>

			<Graph layout={layout} />
		</main>
	);
}
