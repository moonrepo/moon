import { gql } from 'graphql-tag';

export const ProjectInformation = gql`
	query GetProjectInformation {
		workspaceInfo {
			edges {
				id
				source
				target
			}
			nodes {
				id
			}
		}
	}
`;
