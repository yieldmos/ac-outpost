# Osmostake Outpost

Compounding outpost that allows users to specify how they would like to manage their Osmosis staking rewards by percentage. It is intended to be called on a regular basis by Yieldmos so that delegators can manage their rewards in whatever way they would like.

Care has been taken to made the Destination projects modular and easily extendable when new projects/targets become available

## Available Destination Projects

The things you can do with your rewards during any given compounding.

| Destination Project         | Status        | Note                                                                                                                                                                                                                                    |
| --------------------------- | ------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `OSMO Staking`              | `Working`     | Can specify any validator address                                                                                                                                                                                                       |
| `Token Swap`                | `Working`     | Can pick any token that's on Osmosis                                                                                                                                                                                                    |
| `Red Bank Depositing`       | `Working`     | Must specify the denom that you want to lend with. If the lend denom is not OSMO a swap will occur before the lending                                                                                                                   |
| `Red Bank Borrow Pay Back`  | `In Progress` | Payback your loans on Red Bank. Either specifying a whitelist of denoms or selecting all (with an optional list of denoms to pay back preferentially). The order of the denoms listed is the order that the loans will be paid back in. |
| `Red Bank Leverage Looping` | `Working`     | Specify the denom to lever up and optionally a liquidity to value ratio (defaults to 50%).                                                                                                                                              |
| `Osmosis LPs`               | `Not Started` | Must specify the given pool ID                                                                                                                                                                                                          |
| `Red Bank Farm Vaults`      | `Not Started` | Must specify the vault, leverage amount, and borrow amount                                                                                                                                                                              |
