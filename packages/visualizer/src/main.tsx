import './index.css';
import { render } from 'preact';
import { App } from './app';

declare global {
	interface Window {
		GRAPH_DATA: string;
		PAGE_TITLE: string;
	}
}

render(<App />, document.querySelector('#app')!);
