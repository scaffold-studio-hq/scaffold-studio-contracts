import React from "react";
import { Code, Layout, Text } from "@stellar/design-system";

const Home: React.FC = () => (
  <Layout.Content>
    <Layout.Inset>
      <Text as="h1" size="xl">
        Stellar Studio Contracts
      </Text>
      <Text as="p" size="md">
        Build Stellar tokens, NFTs, and governance contracts with reusable
        Soroban factories. Explore the factory contracts, connect a wallet,
        and inspect contract state using the Debugger.
      </Text>

      <Text as="h2" size="lg">
        Factory system
      </Text>
      <Text as="p" size="md">
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
