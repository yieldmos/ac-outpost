# Junostake Outpost

Compounding outpost that allows users to specify how they would like to manage their juno staking rewards by percentage. It is intended to be called on a regular basis by Yieldmos so that delegators can manage their rewards in whatever way they would like.

Care has been taken to made the Destination projects modular and easily extendable when new projects/targets become available

## Available Destination Projects

The things you can do with your rewards during any given compounding.

| Destination Project | Status    | Note                                         |
| ------------------- | --------- | -------------------------------------------- |
| `Juno Staking`      | `working` | Can specify any validator address            |
| `Wynd Staking`      | `working` | Can specify any valid unbonding period       |
| `Neta Staking`      | `working` |                                              |
| `Token Swap`        | `working` | Can pick any token that's on Wyndex          |
| `Wyndex LPs`        | `working` | Must specify the given pool contract address |
