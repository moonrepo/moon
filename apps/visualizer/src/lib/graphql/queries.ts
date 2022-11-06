import { graphql } from '../../generated/graphql';

export const ProjectInformation = graphql(`
	query GetProjectInformation {
		workspaceInfo {
			edges {
				id
				source
				target
			}
			nodes {
				id
				label
			}
		}
	}
`);
