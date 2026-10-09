import { useCallback, useEffect, useState } from "react";
import { useWallet } from "./useWallet";
import { fetchBalance, type Balance } from "../util/wallet";
import { useNotification } from "./useNotification";

const formatter = new Intl.NumberFormat();

const checkFunding = (balances: Balance[]) =>
  balances.some(({ balance }) =>
    !Number.isNaN(Number(balance)) ? Number(balance) > 0 : false,
  );

type WalletBalance = {
  balances: Balance[];
  xlm: string;
  isFunded: boolean;
  isLoading: boolean;
  error: Error | null;
};

export const useWalletBalance = () => {
  const { address } = useWallet();
  const { addNotification } = useNotification();
  const [state, setState] = useState<WalletBalance>({
    balances: [],
    xlm: "-",
    isFunded: false,
    isLoading: false,
    error: null,
  });

  const updateBalance = useCallback(async () => {
    if (!address) return;
    try {
      setState((prev) => ({ ...prev, isLoading: true }));
      const balances = await fetchBalance(address);
      const isFunded = checkFunding(balances);
      const native = balances.find(({ asset_type }) => asset_type === "native");
      setState({
        isLoading: false,
        balances,
        xlm: native?.balance ? formatter.format(Number(native.balance)) : "-",
        isFunded,
        error: null,
      });
    } catch (err) {
      if (err instanceof Error && err.message.match(/not found/i)) {
        addNotification("Could not find this wallet balance. Is your wallet funded?", "error");
        setState({
          isLoading: false,
          balances: [],
          xlm: "-",
          isFunded: false,
          error: new Error("Error fetching balance. Is your wallet funded?"),
        });
      } else {
        addNotification("Unknown error fetching balance.", "error");
        addNotification("Unable to fetch wallet balance. Please try again.", "error");
        setState({
          isLoading: false,
          balances: [],
          xlm: "-",
          isFunded: false,
          error: new Error("Unknown error fetching balance."),
        });
      }
    }
  }, [address, addNotification]);

  useEffect(() => {
    void updateBalance();
  }, [updateBalance]);

  return {
    ...state,
    updateBalance,
  };
};
