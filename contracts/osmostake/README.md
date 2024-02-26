# Osmostake Outpost

Compounding outpost that allows users to specify how they would like to manage OSMO staking rewards by percentage. It is intended to be called on a regular basis by Yieldmos so that delegators can manage their rewards in whatever way they would like.

This contract should be deployed along side an instnace of the `comp_prefs` contract which can store the users' outpost settings and `osmodca` which is another outpost contract.

## Further Reading

To learn about the specific destinations that this outpost should support view the [osmosis-destinations](../../packages/osmosis-destinations/README.md) package as that is where the osmosis destination code is meant to be shared.
