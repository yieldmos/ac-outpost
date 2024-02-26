# Yieldmos Outpost Osmosis DEX Helpers

This is a set of helper functions to be used by the outpost contracts when interacting with the Osmosis protocol. This should increase ergonomics arounds swaps and other interactions.

The intent is to both improve usage of swaps as well as lps

- LP helpers can be found in [osmosis_lp](./src/osmosis_lp.rs).
- Swap helpers can be found in [osmos_swap](./src/osmosis_swap.rs).

Both contain the runtime logic helpers as well as cw-grant-spec grant generators.
