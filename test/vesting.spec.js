const { expect, use } = require("chai");
const { Contract, getAccountByName, polarChai } = require("secret-polar");
const path = require('path');
const fs = require('fs');
const sha256 = require('crypto-js/sha256');
const { MerkleTree } = require('merkletreejs');

use(polarChai);

describe("vesting", () => {
  async function setup() {
    const contract_owner = getAccountByName("account_0");
    const contract = new Contract("snip_20_vesting");
    await contract.parseSchema();

    const whitelistUsers = JSON.parse(fs.readFileSync(path.join(__dirname, "../testdata", "airdrop_external_sig_list.json")));
    console.log(whitelistUsers);

    const leaves = whitelistUsers.map((user) => sha256(user.address + user.amount));
    const tree = new MerkleTree(leaves, sha256, { sort: true });

    const getMerkleProof = (account) => {
      return tree
        .getHexProof(sha256(account.address + account.amount).toString())
        .map((v) => v.replace('0x', ''));
    }

    return { contract_owner, contract, tree, getMerkleProof };
  }

  it("deploy and init", async () => {
    const { contract_owner, contract, tree, getMerkleProof } = await setup();

    const deploy_response = await contract.deploy(contract_owner);
    const merkle_root = tree.getHexRoot().replace('0x', '');
    
    const contract_info = await contract.instantiate(
      {
        "owner": contract_owner.account.address,
        "token_address": contract_owner.account.address,
        "merkle_root": merkle_root
      },
      "Instantiate config",
      contract_owner
    );

    const config = await contract.query.get_config({
      "stage": "1"
    });

    expect(config.merkle_root).to.be.equals(merkle_root);
    expect(config.owner).to.be.equals(contract_owner.account.address);
  
    const current_stage = await contract.query.get_current_stage();
    expect(current_stage).to.be.equals("1");
  });
});
