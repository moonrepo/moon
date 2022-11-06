import './global.css';
import React from 'react';
import ReactDOM from 'react-dom/client';
import { createBrowserRouter, RouterProvider } from 'react-router-dom';
import { Workspace } from './pages/workspace';

const router = createBrowserRouter([{ element: <Workspace />, path: '/project' }]);

ReactDOM.createRoot(document.querySelector('#root')!).render(
	<React.StrictMode>
		<RouterProvider router={router} />
	</React.StrictMode>,
);
