use cosmwasm_std::StdError;

use crate::constants::{ONE_DAY_IN_SECONDS};

pub fn calc_current_vesting_amount(
    current_epoch_index: u64,
    next_claim_epoch_index: u64,
    duration: u64,
    total_amount: u128
) -> Result<(u128, u64), StdError> {
    let passed_days;
    let total_claim_amount;

    let mut latest_next_claim_epoch_index = next_claim_epoch_index;
    let duration_in_days = duration.checked_div(ONE_DAY_IN_SECONDS).unwrap();

    if duration_in_days.eq(&0) {
        total_claim_amount = total_amount;
    } else {
        passed_days = current_epoch_index.checked_sub(next_claim_epoch_index).unwrap().checked_add(1u64).unwrap();
    
        total_claim_amount = total_amount.checked_mul(passed_days as u128).unwrap().checked_div(duration_in_days as u128).unwrap();
        latest_next_claim_epoch_index = current_epoch_index.checked_add(1u64).unwrap();
    }

    Ok((total_claim_amount, latest_next_claim_epoch_index))
}