const { expect, use } = require("chai");
const { Contract, getAccountByName, polarChai } = require("secret-polar");

use(polarChai);

describe("sample_project", () => {
  async function setup() {
    const contract_owner = getAccountByName("account_0");
    const other = getAccountByName("account_0");
    const contract = new Contract("sample-project");
    await contract.parseSchema();

    return { contract_owner, other, contract };
  }

  it("deploy and init", async () => {
    const { contract_owner, other, contract } = await setup();
    const deploy_response = await contract.deploy(contract_owner);

    // const contract_info = await contract.instantiate({"count": 102}, "deploy test", contract_owner);

    // await expect(contract.query.get_count()).to.respondWith({ 'count': 102 });
  });
});
