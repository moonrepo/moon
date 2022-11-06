import { GraphQLClient} from 'graphql-request'

export const client = new GraphQLClient('http://localhost:8000/graphql')
