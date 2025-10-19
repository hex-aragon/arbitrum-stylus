import { createWalletClient, defineChain, http, publicActions } from "viem";
import { privateKeyToAccount } from "viem/accounts";

// Default Nitro Devnode RPC URL
export const DEVNODE_RPC_URL = "http://localhost:8547";
// Default pre-funded private key that exists on Nitro Devnode
// It is already loaded with 100 ETH
export const DEVNODE_PRIVATE_KEY =
  "0xb6b15c8cb491557369f3c7d2c287b053eb229daa9c22138887752191c9520659";

// Define the Nitro Devnode chain object we can pass to other viem functions
export const nitroDevnode = defineChain({
  id: 412346,
  name: "Nitro Devnode",
  nativeCurrency: {
    name: "Ether",
    symbol: "ETH",
    decimals: 18,
  },
  rpcUrls: {
    default: {
      http: [DEVNODE_RPC_URL],
    },
  },
});

// Create a wallet client that can be used to send transactions and make calls to the Nitro Devnode chain
export const walletClient = createWalletClient({
  chain: nitroDevnode,
  transport: http(),
  account: privateKeyToAccount(DEVNODE_PRIVATE_KEY),
}).extend(publicActions);