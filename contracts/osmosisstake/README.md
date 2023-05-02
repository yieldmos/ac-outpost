# Osmostake Outpost

Compounding outpost that allows users to specify how they would like to manage their Osmosis staking rewards by percentage. It is intended to be called on a regular basis by Yieldmos so that delegators can manage their rewards in whatever way they would like.

Care has been taken to made the Destination projects modular and easily extendable when new projects/targets become available

## Available Destination Projects

The things you can do with your rewards during any given compounding.

| Destination Project                                                | Status        | Note                                                       |
| ------------------------------------------------------------------ | ------------- | ---------------------------------------------------------- |
| `OSMO Staking`                                                     | `In Progress` | Can specify any validator address                          |
| `Token Swap`                                                       | `In Progress` | Can pick any token that's on Osmosis                       |
| `Osmosis LPs`                                                      | `Not Started` | Must specify the given pool ID                             |
| `Red Bank Farm Vaults`                                             | `Not Started` | Must specify the vault, leverage amount, and borrow amount |
| `Red Bank Lending`                                                 | `In Progress` | Must specify the denom that you want to lend with.         |
| If the lend denom is not OSMO a swap will occur before the lending |
| `Red Bank Self Repaying Loans`                                     | `Not Started` |                                                            |
