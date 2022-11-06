import React from 'react';
import { Link } from 'react-router-dom';

export const Index = () => (
	<div>
		<div>Links</div>
		<Link to={'/project'}>Project</Link>
	</div>
);
