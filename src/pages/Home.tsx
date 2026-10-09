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
        Build Stellar tokens, NFTs, and governance contracts with reusable
        Soroban factories. Explore the factory contracts, connect a wallet,
        and inspect contract state using the Debugger.
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
        The Master Factory coordinates the Token, NFT, and Governance Factories.
        Each factory deploys specialized contracts rather than requiring a new
        application for every token, collectible, or DAO.
      </Text>

      <Text as="h2" size="lg">
        Tokens, NFTs, and governance
      </Text>
      <Text as="p" size="md">
        The contract suite includes pausable, capped, allowlist, blocklist,
        and vault token implementations; enumerable, royalty, and access-controlled
        NFTs; and Merkle-based governance voting. Browse the source under{" "}
        <Code size="md">contracts/</Code> to review each implementation and its
        configuration.
      </Text>

      <Text as="h2" size="lg">
        Develop and inspect contracts
      </Text>
      <Text as="h2" size="lg">
        Interact with wallets
      <Text as="p" size="md">
        Run <Code size="md">npm run dev</Code> to start the local development
        environment, including <Code size="md">stellar scaffold watch</Code>.
        The watcher recompiles contracts and refreshes generated client bindings
        as the source changes.
      </Text>
      <Text as="p" size="md">
        Use the <Code size="md">Debugger</Code> in the header to explore
        deployed contracts and their callable functions. Connect your Stellar
        wallet first when an interaction needs transaction signing.
      </Text>

      <Text as="h2" size="lg">
        Connect a wallet
      </Text>
      <Text as="p" size="md">
        Use the wallet button above to connect with Stellar Wallet Kit, then
        deploy and operate tokens, NFTs and governance contracts against the
        selected network.
        Build and deploy
      </Text>
      <Text as="p" size="md">
        The repository's <Code size="md">README.md</Code>,{" "}
        <Code size="md">QUICK_START.md</Code>, and{" "}
        <Code size="md">DEPLOYMENT.md</Code> describe the factory architecture,
        local network setup, and contract deployment steps. Build this frontend
        with <Code size="md">npm run build</Code>; its output is in{" "}
        <Code size="md">dist/</Code>.
      </Text>
    </Layout.Inset>
  </Layout.Content>
);

export default Home;
