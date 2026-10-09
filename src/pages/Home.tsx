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
        As an example, here's the <Code size="md">GuessTheNumber</Code>{" "}
        component. Make changes to the contract and the component and see how
        things change!
        Interact with contracts from the frontend
      </Text>
      <Text as="p" size="md">
        Scaffold stellar automatically builds, deploys, and generates frontend
        packages (sometimes called "TypeScript bindings") for each of your
        contracts. You can adjust how it does this in the{" "}
        <Code size="md">environments.toml</Code> file. Import these frontend
        packages like this:
      </Text>
      <pre>
        <Code size="md">import nft_factory from "./contracts/nft_factory";</Code>
      </pre>
      <Text as="p" size="md">
        If your contract emits events, check out the{" "}
        <Code size="md">useSubscription</Code> hook in the{" "}
        <Code size="md">hooks/</Code> folder to listen to them.
      </Text>
      <Text as="p" size="md">
        Each generated client exposes the contract's methods with full
        TypeScript types, so you can call them directly from your components.
        As an example, here's the <Code size="md">GuessTheNumber</Code>{" "}
        component. Make changes to the contract and the component and see how
        things change!
      </Text>
      <Text as="h2" size="lg">
        &lt;GuessTheNumber /&gt;
      </Text>
      <Text as="h2" size="lg">
        Interact with wallets
      </Text>
      <Text as="p" size="md">
        This project is already integrated with Stellar Wallet Kit, and the{" "}
        <Code size="md">useWallet</Code> hook is available for you to use in
        your components. You can use it to connect to get connected account
        information.
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
