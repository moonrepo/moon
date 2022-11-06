import './global.css';
import React from 'react';
import ReactDOM from 'react-dom/client';
import { createBrowserRouter, RouterProvider } from 'react-router-dom';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { Workspace } from './pages/workspace';

const router = createBrowserRouter([{ element: <Workspace />, path: '/project' }]);
const queryClient = new QueryClient();

ReactDOM.createRoot(document.querySelector('#root')!).render(
	<React.StrictMode>
		<QueryClientProvider client={queryClient}>
			<RouterProvider router={router} />
		</QueryClientProvider>
	</React.StrictMode>,
);
