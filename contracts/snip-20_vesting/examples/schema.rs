use std::env::current_dir;
use std::fs::create_dir_all;

use cosmwasm_schema::{export_schema, remove_schemas, schema_for};

use snip_20_vesting::msg::{HandleMsg, InitMsg, QueryMsg, VestingRoundResponse, ContractOwnerResponse};
use snip_20_vesting::state::{VestingRoundState};

fn main() {
    let mut out_dir = current_dir().unwrap();
    out_dir.push("schema");
    create_dir_all(&out_dir).unwrap();
    remove_schemas(&out_dir).unwrap();

    export_schema(&schema_for!(InitMsg), &out_dir);
    export_schema(&schema_for!(HandleMsg), &out_dir);
    export_schema(&schema_for!(QueryMsg), &out_dir);
    export_schema(&schema_for!(ContractOwnerResponse), &out_dir);
    export_schema(&schema_for!(VestingRoundResponse), &out_dir);
    export_schema(&schema_for!(VestingRoundState), &out_dir);
}
