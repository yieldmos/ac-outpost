# Yieldmos Compounding Outposts

The purpose of this repo is to develop and release a set of asset management/compounding contracts predicated upon the functionality enabled by the CSDK Authz module.

## Outpost Architecture

- Outpost contracts do not and should not hold user funds.
- `CompPrefs` refers to the users' selected settings for how they would like the outpost to automate their funds
  - There is a dedicated `comp_prefs` contract meant for local chain storage of user comp prefs
- `Destination Projects`/`DestProjects` refer to the various targets that a user can select from when coming up with their comp prefs.
- Each outpost contract is built for a specific chain and is deployed only there.
  - It is only distinct from it's sibling contracts to avoid grant collisions in the Authz module.
    - For example osmostake and osmodca are seperate so that they each have their own address and thus hold their own unique grants from users and the grants given to one never clash with the other.
- Each chain that we're deploying outposts to should have it's own chain-destinations package where code can be reused between the different outpost contracts
- A key part of the way the system can function is that the contracts contain queries called `GrantSpec` and `RevokeSpec`
  - These queries allow the outposts to dynamically specify the Authz grants/revokes required of the user.
  - See [cw-grant-spec](https://github.com/kakucodes/authzpp/tree/main/packages/grant-spec) for more information on how grants are meant to be declared.
- Care has been taken to made the destination projects modular and easily extendable when new projects/targets become available

## Protocol Usage

The expected flow should go as such:

1. The user should generate their own set of [compounding preferences](./packages/utils/src/comp_prefs.rs) and have them stored wherever they expect to be broadcasting the compounding message (This could be with Yieldmos itself or in the dapp/browser or potentially on the user's own computer for use via the cli).
2. The comp prefs should be given to the outpost in the grants query with the outpost returning a list of the requisite grants that will be needed to fulfilled in order for the outpost be able to later compound for them according to their comp prefs.
3. The user should grant the previously noted Authz grants to the outpost contract's adress.
4. The outpost's compound message can now be called whenever the compounding of rewards should occur.

## Outposts Progress

| Chain ID    | Rewards                        | Status                                           |
| ----------- | ------------------------------ | ------------------------------------------------ |
| `juno-1`    | `staking`                      | [`deployed`](./contracts/junostake/README.md)    |
| `juno-1`    | `wynd staking`                 | [`deployed`](./contracts/wyndstake/README.md)    |
| `juno-1`    | `white whale satellite market` | [`deployed`](./contracts/junowwmarket/README.md) |
| `juno-1`    | `juno dca`                     | [`deployed`](./contracts/junowwmarket/README.md) |
| `osmosis-1` | `staking`                      | [`in progress`](./contracts/osmostake/README.md) |
| `osmosis-1` | `osmo dca`                     | [`in progress`](./contracts/osmodca/README.md)   |
| `migaloo-1` | `whale dca`                    | [`in progress`](./contracts/osmodca/README.md)   |
| `migaloo-1` | `migaloo stake`                | [`in progress`](./contracts/osmodca/README.md)   |

## Packages

| Package Name                                                          | Description                                                                                                   |
| --------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------- |
| [deploy](./packages/deploy/README.md)                                 | [cw-orchestrator](https://orchestrator.abstract.money) scripts for deploying the contracts                    |
| [juno-destinations](./packages/juno-destinations/README.md)           | Types, message generators, and grant generators for Juno specific destinations                                |
| [migaloo-destinations](./packages/migaloo-destinations/README.md)     | Types, message generators, and grant generators for Migaloo specific destinations                             |
| [osmosis-destinations](./packages/osmosis-destinations/README.md)     | Types, message generators, and grant generators for Osmosis specific destinations                             |
| [osmosis-helpers](./packages/osmosis-helpers/README.md)               | Helpers for interacting with Osmosis DEX (both lping and swapping)                                            |
| [sail-destinations](./packages/sail-destinations/README.md)           | Types, message generators, and grant generators for SAIL specific destinations                                |
| [terraswap-helpers](./packages/terraswap-helpers/README.md)           | Helpers for interacting with Terraswap DEX                                                                    |
| [universal-destinations](./packages/universal-destinations/README.md) | Types, message generators, and grant generators for destinations that are expected to be use on every outpost |
| [utils](./packages/utils/README.md)                                   | Base utilities for building outposts                                                                          |
| [wynd-helpers](./packages/wynd-helpers/README.md)                     | Helpers for interacting with Wynd Staking and Wyndex                                                          |
