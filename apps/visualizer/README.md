# Visualizer

The UI that is displayed in the browser when a user runs `moon visualize`. It fetches data from a
Rocket server running in the backend, via a graphql endpoint.

## Tech stack

This is a react app that uses the following stack:

- [React][1]: For the UI rendering
- [React query][2]: For fetching data from the backend
- [Graphql codegen][3]: For generating typescript types from the graphql schema
- [Cytoscape][4]: For rendering the graph

## Working

When running in development mode, the backend server should be run using
`./target/debug/moon visualize`, and the frontend using `./target/debug/moon run visualizer:dev`. In
release, the frontend is built and placed in the `dist` folder. This output is then embedded using
[`rust-embed`][5] statically into the `moon` CLI and served directly by the Rocket server.

[1]: https://reactjs.org
[2]: https://tanstack.com/query/v4/docs/overview
[3]: https://www.the-guild.dev/graphql/codegen
[4]: https://js.cytoscape.org
[5]: https://docs.rs/rust_embed
