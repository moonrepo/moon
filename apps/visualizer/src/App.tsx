import React, { useState } from 'react';
import { Flow } from './components/Flow';

function App() {
	const [count, setCount] = useState(0);

	return (
		<div className="App">
			<div>Hello</div>
			<p>You have clicked {count} times</p>
			<button onClick={() => void setCount(count + 1)}>Click</button>
			<Flow />
		</div>
	);
}

// eslint-disable-next-line import/no-default-export
export default App;
