pub enum WyndStakeReceive {
    /// https://github.com/wynddao/wynddex/blob/main/packages/wyndex/src/stake.rs#L36
    Delegate {
        /// Unbonding period in seconds
        unbonding_period: u64,
        /// If set, the staked assets will be assigned to the given address instead of the sender
        delegate_as: Option<String>,
    },
}
