import './app.css';
import { Graph } from './components/Graph';

export function App() {
	return (
		<main>
			<h2 className="m-0 p-4 text-3xl font-extrabold sm:text-4xl">{window.PAGE_TITLE}</h2>
			<Graph />
		</main>
	);
}
