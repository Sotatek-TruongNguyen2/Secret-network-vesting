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
    const other = getAccountByName("account_1");
    const malicious = getAccountByName("account_2");
    const contract = new Contract("snip-20_vesting");
    await contract.parseSchema();

    const whitelistUsers = JSON.parse(fs.readFileSync(path.join(__dirname, "../testdata", "airdrop_external_sig_list.json")));

    const leaves = whitelistUsers.map((user) => sha256(user.address + user.amount));
    const tree = new MerkleTree(leaves, sha256, { sort: true });

    const getMerkleProof = (account) => {
      return tree
        .getHexProof(sha256(account.address + account.amount).toString())
        .map((v) => v.replace('0x', ''));
    }

    await contract.deploy(contract_owner);
    const merkle_root = tree.getHexRoot().replace('0x', '');

    await contract.instantiate(
      {
        "owner": contract_owner.account.address,
        "contract_status": 0
      },
      "Instantiate config",
      contract_owner
    );

    return { other, contract_owner, malicious, contract, getMerkleProof, merkle_root };
  }

  it("Vesting contracts should be deployed successful!", async () => {
    const { contract } = await setup();
    expect(contract.contractAddress).to.be.not.null;
  });

  it("contract owner able to register new vesting round!", async () => {
    const { other, contract_owner, contract } = await setup();

    const whitelistUsers = JSON.parse(fs.readFileSync(path.join(__dirname, "../testdata", "airdrop_external_sig_list.json")));

    const leaves = whitelistUsers.map((user) => sha256(user.address + user.amount));
    const anotherTree = new MerkleTree(leaves, sha256, { sort: true });

    const another_merkle_root = anotherTree.getHexRoot().replace('0x', '');

    const receipt = await contract.executeMsg(
      "register_new_vesting_round",
      {
        "owner": other.account.address,
        "token_address": other.account.address,
        "merkle_root": another_merkle_root
      },
      contract_owner
    )

    // console.log(receipt.logs[0].events[1].attributes);

    const current_stage = await contract.query.get_current_stage();
    expect(current_stage).to.be.equals("1");

    const config = await contract.query.get_config({
      "stage": "1"
    });

    expect(config.stage).to.be.equals("1");
    expect(config.merkle_root).to.be.equals(another_merkle_root);
  });

  it("non contract owner able to register new vesting round!", async () => {
    const { other, contract } = await setup();

    const whitelistUsers = JSON.parse(fs.readFileSync(path.join(__dirname, "../testdata", "airdrop_external_sig_list.json")));

    const leaves = whitelistUsers.map((user) => sha256(user.address + user.amount));
    const anotherTree = new MerkleTree(leaves, sha256, { sort: true });

    const another_merkle_root = anotherTree.getHexRoot().replace('0x', '');

    await expect(
      contract.executeMsg(
        "register_new_vesting_round",
        {
          "owner": other.account.address,
          "token_address": other.account.address,
          "merkle_root": another_merkle_root
        },
        other
      )
    ).to.be.revertedWith("This is an admin command. Admin commands can only be run from admin address")

    await expect(contract.query.get_config({
      "stage": "1"
    })).to.be.revertedWith("No configuration for this stage");
  });

  it("contract owner able to transfer ownership", async () => {
    const { other, contract, contract_owner } = await setup();

    await contract.executeMsg(
      "grant_contract_owner",
      {
        "new_admin": other.account.address
      },
      contract_owner
    )

    const granted_contract_owner = await contract.query.granted_contract_owner();
    expect(granted_contract_owner).to.be.equals(other.account.address);
  });

  it("granted contract owner able to claim ownership", async () => {
    const { other, contract, contract_owner } = await setup();

    await contract.executeMsg(
      "grant_contract_owner",
      {
        "new_admin": other.account.address
      },
      contract_owner
    )

    let granted_contract_owner = await contract.query.granted_contract_owner();
    expect(granted_contract_owner).to.be.equals(other.account.address);

    await contract.executeMsg(
      "claim_contract_owner",
      {},
      other
    )
    granted_contract_owner = await contract.query.granted_contract_owner();
    expect(granted_contract_owner).not.to.be.equals(other.account.address);

    const new_contract_owner = await contract.query.contract_owner();
    expect(new_contract_owner).to.be.equals(other.account.address);
  });

  it("non granted contract owner not able to claim ownership", async () => {
    const { other, malicious, contract, contract_owner } = await setup();

    await contract.executeMsg(
      "grant_contract_owner",
      {
        "new_admin": other.account.address
      },
      contract_owner
    )

    let granted_contract_owner = await contract.query.granted_contract_owner();
    expect(granted_contract_owner).to.be.equals(other.account.address);

    await expect(
      contract.executeMsg(
        "claim_contract_owner",
        {},
        malicious
      )
    ).to.be.revertedWith("This is a granted admin command. Granted admin commands can only be run from granted admin address")
  });

  it("contract owner able to revoke granted ownership", async () => {
    const { other, contract, contract_owner } = await setup();

    await contract.executeMsg(
      "grant_contract_owner",
      {
        "new_admin": other.account.address
      },
      contract_owner
    )

    let granted_contract_owner = await contract.query.granted_contract_owner();
    expect(granted_contract_owner).to.be.equals(other.account.address);

    await contract.executeMsg(
      "revoke_granted_contract_owner",
      {},
      contract_owner
    )

    granted_contract_owner = await contract.query.granted_contract_owner();
    expect(granted_contract_owner).not.to.be.equals(other.account.address);
  
    await expect(
      contract.executeMsg(
        "claim_contract_owner",
        {},
        other
      )
    ).to.be.revertedWith("This is a granted admin command. Granted admin commands can only be run from granted admin address")
  });
});
