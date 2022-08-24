const { expect, use } = require("chai");
const { Contract, getAccountByName, polarChai } = require("secret-polar");
const path = require('path');
const fs = require('fs');
const sha256 = require('crypto-js/sha256');
const { MerkleTree } = require('merkletreejs');
const { BigNumber } = require('bignumber.js');

use(polarChai);

describe("vesting", () => {
  async function setup() {
    const TOTAL_LOCKED_AMOUNT = new BigNumber(100000).multipliedBy(new BigNumber(10).pow(18));
   
    const contract_owner = getAccountByName("account_0");
    const other = getAccountByName("account_1");
    const malicious = getAccountByName("account_2");

    const WHITELIST_USERS = [
      {
        address: other.account.address,
        amount: "100",
        tge: "2000",
        cliff: 5,
        duration: 30,
        stage: "1",
        startAt: Math.floor(new Date().getTime() / 1000) 
      },
      {
        address: contract_owner.account.address,
        amount: "100",
        tge: "2000",
        cliff: 5,
        duration: 30,
        stage: "1",
        startAt: Math.floor(new Date().getTime() / 1000)
      },
    ];

    const contract = new Contract("snip_20_vesting");
    const snip20_token = new Contract("snip_20");

    await contract.parseSchema();
    await snip20_token.parseSchema();


    await contract.deploy(contract_owner, {
      amount: [{ amount: "750000", denom: "uscrt" }],
      gas: "3000000",
    });
    await snip20_token.deploy(contract_owner, {
      amount: [{ amount: "750000", denom: "uscrt" }],
      gas: "3000000",
    });

    await contract.instantiate(
      {
        "owner": contract_owner.account.address,
        "contract_status": 0
      },
      "Instantiate config",
      contract_owner
    );

    await snip20_token.instantiate(
      {
        "name": "My snip20",
        "admin": contract_owner.account.address,
        "symbol": "SPL",
        "decimals": 18,
        "initial_balances": [
          {
            address: contract_owner.account.address,
            amount: TOTAL_LOCKED_AMOUNT.toFixed()
          }
        ],
        "prng_seed": Buffer.from(
          Buffer.from('My random seed').toString('base64'),
          'utf-8'
        ).toString(),
        "config": {
          enable_mint: true
        }
      },
      "Instantiate SNIP20 token",
      contract_owner,
    )

    return { TOTAL_LOCKED_AMOUNT, WHITELIST_USERS, other, snip20_token, contract_owner, malicious, contract };
  }

  xit("Vesting contracts should be deployed successful!", async () => {
    const { contract } = await setup();
    expect(contract.contractAddress).to.be.not.null;
  });

  xdescribe("Ownership", async function () {
    it("contract owner able to register new vesting round!", async () => {
      const { other, snip20_token, contract_owner, contract, WHITELIST_USERS } = await setup();

      const leaves = WHITELIST_USERS.map((user) => sha256(user.address + user.amount));
      const anotherTree = new MerkleTree(leaves, sha256, { sort: true });

      const another_merkle_root = anotherTree.getHexRoot().replace('0x', '');

      const receipt = await contract.executeMsg(
        "register_new_vesting_round",
        {
          "owner": other.account.address,
          "token_address": snip20_token.contractAddress,
          "token_code_hash": snip20_token.contractCodeHash,
          "distribution": null,
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
      const { other, contract, WHITELIST_USERS } = await setup();

      const leaves = WHITELIST_USERS.map((user) => sha256(user.address + user.amount));
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
  })

  describe("Claim", async function () {
    async function claimSetup() {
      const { WHITELIST_USERS, TOTAL_LOCKED_AMOUNT, other, snip20_token, contract_owner, contract } = await setup();

      const leaves = WHITELIST_USERS.map((user) => sha256(user.address + user.stage + user.amount + user.tge + user.startAt + user.duration + user.cliff));
      const tree = new MerkleTree(leaves, sha256, { sort: true });

      const merkle_root = tree.getHexRoot().replace('0x', '');

      const getMerkleProof = (accountIndex) => {
        const account = WHITELIST_USERS[accountIndex];
        return tree
          .getHexProof(sha256(account.address + account.stage + account.amount + account.tge + account.startAt + account.duration + account.cliff).toString())
          .map((v) => v.replace('0x', ''));
      }

      const receipt = await contract.executeMsg(
        "register_new_vesting_round",
        {
          "owner": other.account.address,
          "token_address": snip20_token.contractAddress,
          "token_code_hash": snip20_token.contractCodeHash,
          "distribution": null,
          "merkle_root": merkle_root
        },
        contract_owner
      )

      await snip20_token.executeMsg(
        "increase_allowance",
        {
          "amount": TOTAL_LOCKED_AMOUNT.toFixed(),
          "spender": contract.contractAddress
        },
        contract_owner
      );

      return {
        WHITELIST_USERS,
        getMerkleProof,
        contract_owner,
        contract,
        snip20_token,
        other
      }
    }
    
    xit("Whitelist user able to starts claiming vesting tokens", async () => {
      const { WHITELIST_USERS, getMerkleProof, contract_owner, contract, other } = await claimSetup();
      const user_index = 0;
      const proofs = getMerkleProof(user_index);

      const receipt = await contract.executeMsg(
        "claim",
        {
          "proof": proofs,
          "start_at": WHITELIST_USERS[user_index].startAt,
          "stage": WHITELIST_USERS[user_index].stage,
          "amount": WHITELIST_USERS[user_index].amount,
          "cliff": WHITELIST_USERS[user_index].cliff,
          "duration": WHITELIST_USERS[user_index].duration,
          "tge": WHITELIST_USERS[user_index].tge
        },
        other
      );

      console.log(receipt.logs[0].events[1].attributes);
    });

    it("Whitelist user not able to modify claim proofs", async () => {
      const { WHITELIST_USERS, getMerkleProof, contract_owner, contract, other } = await claimSetup();
      const user_index = 0;
      const proofs = getMerkleProof(user_index);

      await expect(
        contract.executeMsg(
          "claim",
          {
            "proof": proofs,
            "start_at": WHITELIST_USERS[user_index].startAt,
            "stage": WHITELIST_USERS[user_index].stage,
            "amount": "3000",
            "cliff": WHITELIST_USERS[user_index].cliff,
            "duration": WHITELIST_USERS[user_index].duration,
            "tge": WHITELIST_USERS[user_index].tge
          },
          other
        )
      ).to.be.revertedWith("Proof verification failed!");
    });
  })
});
