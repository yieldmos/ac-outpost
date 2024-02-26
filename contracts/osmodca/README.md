# Osmodca Outpost

Compounding outpost that allows users to specify how they would like to manage their liquid OSMO balance. Users can select how much OSMO they would like applied to each execution and how frequently they would like it called (as well as how long they are authorizing the outpost to operate on their account for). It is intended to be called on a regular basis by Yieldmos so that delegators can manage their rewards in whatever way they would like.

This contract should be deployed along side an instnace of the `comp_prefs` contract which can store the users' outpost settings and `osmostake` which is another outpost contract.

## Further Reading

To learn about the specific destinations that this outpost should support view the [osmosis-destinations](../../packages/osmosis-destinations/README.md) package as that is where the osmosis destination code is meant to be shared.
