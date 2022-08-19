const accounts = [
  {
    name: 'account_0',
    address: 'secret1pt7vpkzpm7f6n6nvcvx096gfnln4qkawhpfk8g',
    mnemonic: 'salad holiday elevator exile marble casual job extend sail wedding feed language electric gloom orphan night input oval differ mango shock year cake saddle'
  },
  // {
  //   name: 'account_0',
  //   address: 'secret1fc3fzy78ttp0lwuujw7e52rhspxn8uj52zfyne',
  //   mnemonic: 'jelly shadow frog dirt dragon use armed praise universe win jungle close inmate rain oil canvas beauty pioneer chef soccer icon dizzy thunder meadow',
  // },
  // {
  //   name: 'account_1',
  //   address: 'secret1ldjxljw7v4vk6zhyduywh04hpj0jdwxsmrlatf',
  //   mnemonic: 'word twist toast cloth movie predict advance crumble escape whale sail such angry muffin balcony keen move employ cook valve hurt glimpse breeze brick',
  // },
  // {
  //   name: 'account_2',
  //   address: 'secret1ajz54hz8azwuy34qwy9fkjnfcrvf0dzswy0lqq',
  //   mnemonic: 'chair love bleak wonder skirt permit say assist aunt credit roast size obtain minute throw sand usual age smart exact enough room shadow charge',
  // },
];

const networks = {
  localnet: {
    endpoint: 'http://localhost:1317/',
    chainId: "secretdev-11",
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
    version: "1.65.0",
  },
};
