import React from "react";
import { Code, Layout, Text } from "@stellar/design-system";

const Home: React.FC = () => (
  <Layout.Content>
    <Layout.Inset>
      <Text as="h1" size="xl">
        Stellar Studio Contracts
      </Text>
      <Text as="p" size="md">
        Deploy and command Stellar tokens, NFTs, and DAOs through conversation.
        This is the frontend for the Soroban factory system: a master factory
        that deploys specialized token, NFT and governance factories, which in
        turn deploy and manage contract instances on-chain.
      </Text>

      <Text as="h2" size="lg">
        Factory system
      </Text>
      <Text as="p" size="md">
        <Code size="md">MasterFactory</Code> deploys three specialized
        factories. <Code size="md">TokenFactory</Code> deploys fungible tokens
        (pausable, capped, allowlist, blocklist and vault variants),{" "}
        <Code size="md">NFTFactory</Code> deploys enumerable, access-control and
        royalty-bearing NFT collections, and{" "}
        <Code size="md">GovernanceFactory</Code> deploys Merkle-proof voting
        DAOs.
      </Text>

      <Text as="h2" size="lg">
        Explore the contracts
      </Text>
      <Text as="p" size="md">
        Browse the contract sources under <Code size="md">contracts/</Code>,
        inspect the generated TypeScript clients under{" "}
        <Code size="md">packages/</Code>, and use the{" "}
        <Code size="md">&lt;/&gt; Debugger</Code> in the top right to simulate
        and invoke contract functions from the browser.
      </Text>

      <Text as="h2" size="lg">
        Connect a wallet
      </Text>
      <Text as="p" size="md">
        Use the wallet button above to connect with Stellar Wallet Kit, then
        deploy and operate tokens, NFTs and governance contracts against the
        selected network.
      </Text>
    </Layout.Inset>
  </Layout.Content>
);

export default Home;
