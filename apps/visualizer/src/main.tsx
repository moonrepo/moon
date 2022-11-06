import './global.css';
import React from 'react';
import ReactDOM from 'react-dom/client';
import { createBrowserRouter, RouterProvider } from 'react-router-dom';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { Index } from './pages';
import { Project } from './pages/project';

const router = createBrowserRouter([
	{ element: <Index />, path: '/' },
	{ element: <Project />, path: '/project' },
]);
const queryClient = new QueryClient();

ReactDOM.createRoot(document.querySelector('#root')!).render(
	<React.StrictMode>
		<QueryClientProvider client={queryClient}>
			<RouterProvider router={router} />
		</QueryClientProvider>
	</React.StrictMode>,
);
