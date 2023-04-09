### Yieldmos Outpost Utils

This is a set of general utility functions to be used by the actual outpost contracts

Please note that this is still a pre-release version and breaking changes are expected to be frequent

## Example Compounding Prefs

```json
{
  "msg": {
    "compound": {
      "comp_prefs": {
        "relative": [
          {
            "destination": {
              "neta_staking": {}
            },
            "amount": "250000000000000000"
          },
          {
            "destination": {
              "wynd_staking": {
                "bonding_period": "seven_hundred_thirty_days"
              }
            },
            "amount": "250000000000000000"
          },
          {
            "destination": {
              "juno_staking": {
                "validator_address": "junovaloper1m55p4c956dawa95uhzz027p4pwm3fedfkdtnyj"
              }
            },
            "amount": "250000000000000000"
          },
          {
            "destination": {
              "token_swap": {
                "target_denom": {
                  "native": "ibc/EAC38D55372F38F1AFD68DF7FE9EF762DCF69F26520643CF3F9D292A738D8034"
                }
              }
            },
            "amount": "250000000000000000"
          }
        ]
      },
      "delegator_address": "juno14r9pzdtdza6ma9cs2mznqrc4zqjwflsj3dl5tl"
    }
  },
  "funds": []
}
```
