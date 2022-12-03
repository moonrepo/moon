import './app.css';
import { Graph } from './components/Graph';

export function App() {
	return (
		<main className="my-10">
			<h2 className="text-5xl text-center underline capitalize">{window.PAGE_TITLE} Graph</h2>
			<Graph />
		</main>
	);
}
