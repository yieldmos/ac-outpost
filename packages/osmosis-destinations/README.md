# Osmosis Destinations

This is the space to store specific logic related to the individual destinations that should be made accessable exlusively on the Osmosis Outposts.

## Current Destinations

- An up to date list of the currently supported destinations can be viewed in the [`OsmosisDestinationProject`](./src/comp_prefs.rs) enum.
- Implementations of the destination projects (aka `msg gens`) can be seen in all of the different `*-destinations` packages but most notibly [Osmosis `dest_project_gen`](./src/dest_project_gen.rs).
- Grant generator implementations for the `osmosis-destinations` can be found in [Osmosis `grants`](./src/grants.rs).
