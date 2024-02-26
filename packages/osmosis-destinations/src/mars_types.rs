use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Coin, Decimal, Uint128};

#[cw_serde]
pub enum RedBankExecuteMsgs {
    /// Update user's position on their credit account
    UpdateCreditAccount {
        account_id: String,
        actions: Vec<RedBankAction>,
    },
    /// Repay debt on behalf of an account, funded from wallet. Must send exactly one coin in message funds.
    /// Allows repaying debts of assets that have been de-listed from credit manager.
    RepayFromWallet { account_id: String },
}

#[cw_serde]
pub enum RedBankAction {
    /// Deposit coin of specified denom and amount. Verifies if the correct amount is sent with transaction.
    Deposit(Coin),
    /// Withdraw coin of specified denom and amount
    Withdraw(ActionCoin),
    /// Borrow coin of specified amount from Red Bank
    Borrow(Coin),
    /// Lend coin to the Red Bank
    Lend(ActionCoin),
    /// Reclaim the coins that were lent to the Red Bank.
    Reclaim(ActionCoin),
    /// For assets lent to the Red Bank, some can accumulate incentive rewards.
    /// This message claims all of them adds them to account balance.
    ClaimRewards {},
    /// Repay coin of specified amount back to Red Bank. If `amount: AccountBalance` is passed,
    /// the repaid amount will be the minimum between account balance for denom and total owed.
    /// The sender will repay on behalf of the recipient account. If 'recipient_account_id: None',
    /// the sender repays to its own account.
    Repay {
        recipient_account_id: Option<String>,
        coin: ActionCoin,
    },
    // /// Deposit coins into vault strategy
    // /// If `coin.amount: AccountBalance`, Rover attempts to deposit the account's entire balance into the vault
    // EnterVault {
    //     vault: VaultUnchecked,
    //     coin: ActionCoin,
    // },
    // /// Withdraw underlying coins from vault
    // ExitVault {
    //     vault: VaultUnchecked,
    //     amount: Uint128,
    // },
    // /// Requests unlocking of shares for a vault with a required lock period
    // RequestVaultUnlock {
    //     vault: VaultUnchecked,
    //     amount: Uint128,
    // },
    // /// Withdraws the assets for unlocking position id from vault. Required time must have elapsed.
    // ExitVaultUnlocked { id: u64, vault: VaultUnchecked },
    // /// Pay back debt of a liquidatable rover account for a via liquidating a specific type of the position.
    // Liquidate {
    //     /// The credit account id of the one with a liquidation threshold health factor 1 or below
    //     liquidatee_account_id: String,
    //     /// The coin they wish to acquire from the liquidatee (amount returned will include the bonus)
    //     debt_coin: Coin,
    //     /// Position details to be liquidated
    //     request: LiquidateRequest<VaultUnchecked>,
    // },
    // /// Perform a swapper with an exact-in amount. Requires slippage allowance %.
    // /// If `coin_in.amount: AccountBalance`, the accounts entire balance of `coin_in.denom` will be used.
    // SwapExactIn {
    //     coin_in: ActionCoin,
    //     denom_out: String,
    //     slippage: Decimal,
    //     route: Option<SwapperRoute>,
    // },
    /// Add Vec<Coin> to liquidity pool in exchange for LP tokens.
    /// Slippage allowance (%) is used to calculate the minimum amount of LP tokens to receive.
    ProvideLiquidity {
        coins_in: Vec<ActionCoin>,
        lp_token_out: String,
        slippage: Decimal,
    },
    /// Send LP token and withdraw corresponding reserve assets from pool.
    /// If `lp_token.amount: AccountBalance`, the account balance of `lp_token.denom` will be used.
    /// /// Slippage allowance (%) is used to calculate the minimum amount of reserve assets to receive.
    WithdrawLiquidity {
        lp_token: ActionCoin,
        slippage: Decimal,
    },
    /// Refunds all coin balances back to user wallet
    RefundAllCoinBalances {},
}

#[cw_serde]
pub enum ActionAmount {
    Exact(Uint128),
    AccountBalance,
}

impl ActionAmount {
    pub fn value(&self) -> Option<Uint128> {
        match self {
            ActionAmount::Exact(amt) => Some(*amt),
            ActionAmount::AccountBalance => None,
        }
    }
}

#[cw_serde]
pub struct ActionCoin {
    pub denom: String,
    pub amount: ActionAmount,
}

impl From<&Coin> for ActionCoin {
    fn from(value: &Coin) -> Self {
        Self {
            denom: value.denom.to_string(),
            amount: ActionAmount::Exact(value.amount),
        }
    }
}
