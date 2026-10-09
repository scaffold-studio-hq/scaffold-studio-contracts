import React from "react";
import { Icon } from "@stellar/design-system";
import { useWallet } from "../hooks/useWallet";
import { stellarNetwork } from "../contracts/util";

// Format network name with first letter capitalized
const formatNetworkName = (name: string) =>
  // TODO: This is a workaround until @creit-tech/stellar-wallets-kit uses the new name for a local network.
  name === "STANDALONE"
    ? "Local"
    : name.charAt(0).toUpperCase() + name.slice(1).toLowerCase();

const appNetwork = formatNetworkName(stellarNetwork);

const bgColor = "#F0F2F5";
const textColor = "#4A5362";

const NetworkPill: React.FC = () => {
  const { network, address } = useWallet();

  // Check if there's a network mismatch
  const walletNetwork = formatNetworkName(network ?? "");
  const isNetworkMismatch = walletNetwork !== appNetwork;

  let title = "";
  let statusDescription = `Wallet is connected to ${appNetwork}.`;
  let color = "#2ED06E";
  if (!address) {
    title = "Connect your wallet using this network.";
    statusDescription = `Connect your wallet using the ${appNetwork} network.`;
    color = "#C1C7D0";
  } else if (isNetworkMismatch) {
    title = `Wallet is on ${walletNetwork}, connect to ${appNetwork} instead.`;
    statusDescription = `Wallet is on ${walletNetwork}, connect to ${appNetwork} instead.`;
    color = "#FF3B30";
  }

  // Include the mismatch message in the accessible name so assistive
  // technology announces it, not just the network name.
  const ariaLabel = `${appNetwork} network. ${statusDescription}`;
  const accessibleStatus = !address
    ? `App network: ${appNetwork}. Connect your wallet using this network.`
    : isNetworkMismatch
      ? `Network mismatch: wallet is on ${walletNetwork}; connect to ${appNetwork} instead.`
      : `Wallet network matches the app network: ${appNetwork}.`;

  return (
    <div
      role="status"
      aria-label={ariaLabel}
      aria-label={accessibleStatus}
      style={{
        backgroundColor: bgColor,
        color: textColor,
        padding: "4px 10px",
        borderRadius: "16px",
        fontSize: "12px",
        fontWeight: "bold",
        display: "flex",
        alignItems: "center",
        gap: "4px",
        cursor: isNetworkMismatch ? "help" : "default",
      }}
      title={title}
    >
      <span aria-hidden="true">
        <Icon.Circle color={color} />
      </span>
      {appNetwork}
    </div>
  );
};

export default NetworkPill;
