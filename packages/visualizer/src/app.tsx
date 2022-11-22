import './app.css';
import { useEffect, useState } from 'preact/hooks';
import { Graph } from './components/Graph';

export function App() {
	const [title, setTitle] = useState('');

	useEffect(() => void setTitle(window.PAGE_TITLE), []);

	return (
		<main className="my-10">
			<h2 className="text-5xl text-center underline capitalize">{title} Graph</h2>
			<Graph />
		</main>
	);
}
