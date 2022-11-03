import { useState } from 'react';

function App() {
	const [count, setCount] = useState(0);

	return (
		<div className="App">
			<div>Hello</div>
			<p>You have clicked {count} times</p>
			<button onClick={() => setCount(count + 1)}>Click</button>
		</div>
	);
}

export default App;
