const accounts = [
  {
    name: 'account_0',
    address: 'secret1pt7vpkzpm7f6n6nvcvx096gfnln4qkawhpfk8g',
    mnemonic: 'salad holiday elevator exile marble casual job extend sail wedding feed language electric gloom orphan night input oval differ mango shock year cake saddle'
  },
  {
    name: 'account_1',
    address: 'secret1fc3fzy78ttp0lwuujw7e52rhspxn8uj52zfyne',
    mnemonic: 'jelly shadow frog dirt dragon use armed praise universe win jungle close inmate rain oil canvas beauty pioneer chef soccer icon dizzy thunder meadow',
  }
];

const networks = {
  localnet: {
    endpoint: 'http://localhost:1337/',
    accounts,
  },
  // Pulsar-2
  testnet: {
    endpoint: 'http://testnet.securesecrets.org:1317/',
    chainId: 'pulsar-2',
    accounts,
  },
  development: {
    endpoint: 'tcp://0.0.0.0:26656',
    chainId: 'enigma-pub-testnet-3',
    types: {},
    accounts
  },
  // Supernova Testnet
  supernova: {
    endpoint: 'http://bootstrap.supernova.enigma.co:1317',
    chainId: 'supernova-2',
    accounts: accounts,
    types: {},
    fees: {
      upload: {
          amount: [{ amount: "500000", denom: "uscrt" }],
          gas: "2000000",
      },
      init: {
          amount: [{ amount: "125000", denom: "uscrt" }],
          gas: "500000",
      },
    }
  }
};

module.exports = {
  networks: {
    default: networks.testnet,
    localnet: networks.localnet,
    testnet: networks.testnet,
    development: networks.development,
  },
  mocha: {
    timeout: 60000
  },
  rust: {
    version: "1.55.0",
  },
};
