import {
  createContext,
  useEffect,
  useMemo,
  useRef,
  useState,
  useTransition,
} from "react";
import { wallet } from "../util/wallet";
import storage from "../util/storage";
import { useNotification } from "../hooks/useNotification";

export interface WalletContextType {
  address?: string;
  network?: string;
  networkPassphrase?: string;
  isPending: boolean;
  signTransaction?: typeof wallet.signTransaction;
}

const initialState = {
  address: undefined,
  network: undefined,
  networkPassphrase: undefined,
};

const POLL_INTERVAL = 1000;

export const WalletContext = // eslint-disable-line react-refresh/only-export-components
  createContext<WalletContextType>({ isPending: true });

export const WalletProvider = ({ children }: { children: React.ReactNode }) => {
  const [state, setState] =
    useState<Omit<WalletContextType, "isPending">>(initialState);
  const [isPending, startTransition] = useTransition();
  const popupLock = useRef(false);
  const errorNotified = useRef(false);
  const { addNotification } = useNotification();
  const signTransaction = wallet.signTransaction.bind(wallet);

  const nullify = () => {
    updateState(initialState);
    storage.setItem("walletId", "");
    storage.setItem("walletAddress", "");
    storage.setItem("walletNetwork", "");
    storage.setItem("networkPassphrase", "");
  };

  const updateState = (newState: Omit<WalletContextType, "isPending">) => {
    setState((prev: Omit<WalletContextType, "isPending">) => {
      if (
        prev.address !== newState.address ||
        prev.network !== newState.network ||
        prev.networkPassphrase !== newState.networkPassphrase
      ) {
        return newState;
      }
      return prev;
    });
  };

  const updateCurrentWalletState = async () => {
    // There is no way, with StellarWalletsKit, to check if the wallet is
    // installed/connected/authorized. We need to manage that on our side by
    // checking our storage item.
    const walletId = storage.getItem("walletId");
    const walletNetwork = storage.getItem("walletNetwork");
    const walletAddr = storage.getItem("walletAddress");
    const passphrase = storage.getItem("networkPassphrase");

    if (
      !state.address &&
      walletAddr !== null &&
      walletNetwork !== null &&
      passphrase !== null
    ) {
      updateState({
        address: walletAddr,
        network: walletNetwork,
        networkPassphrase: passphrase,
      });
    }

    if (!walletId) {
      nullify();
    } else {
      if (popupLock.current) return;
      // If our storage item is there, then we try to get the user's address &
      // network from their wallet. Note: `getAddress` MAY open their wallet
      // extension, depending on which wallet they select!
      try {
        popupLock.current = true;
        wallet.setWallet(walletId);
        if (walletId !== "freighter" && walletAddr !== null) return;
        const [a, n] = await Promise.all([
          wallet.getAddress(),
          wallet.getNetwork(),
        ]);

        // A successful poll clears the previous error so a later failure can
        // notify the user again.
        errorNotified.current = false;

        if (!a.address) storage.setItem("walletId", "");
        if (
          a.address !== state.address ||
          n.network !== state.network ||
          n.networkPassphrase !== state.networkPassphrase
        ) {
          storage.setItem("walletAddress", a.address);
          updateState({ ...a, ...n });
        }
      } catch (e) {
        // If `getNetwork` or `getAddress` throw errors... sign the user out???
        nullify();
        // Surface the error instead of throwing so the app keeps running, but
        // only once per failure burst so the polling loop cannot spam the user.
        if (!errorNotified.current) {
          errorNotified.current = true;
          addNotification(
            e instanceof Error
              ? `Wallet connection error: ${e.message}`
              : "Wallet connection error. Please check your wallet extension.",
            "error",
          );
        }
      } finally {
        popupLock.current = false;
      }
    }
  };

  useEffect(() => {
    let timer: NodeJS.Timeout;
    let isMounted = true;

    // Create recursive polling function to check wallet state continuously
    const pollWalletState = async () => {
      if (!isMounted) return;

      await updateCurrentWalletState();

      if (isMounted) {
        timer = setTimeout(() => void pollWalletState(), POLL_INTERVAL);
      }
    };

    // Get the wallet address when the component is mounted for the first time
    startTransition(async () => {
      await updateCurrentWalletState();
      // Start polling after initial state is loaded

      if (isMounted) {
        timer = setTimeout(() => void pollWalletState(), POLL_INTERVAL);
      }
    });

    // Clear the timeout and stop polling when the component unmounts
    return () => {
      isMounted = false;
      if (timer) clearTimeout(timer);
    };
  }, [state]); // eslint-disable-line react-hooks/exhaustive-deps -- it SHOULD only run once per component mount

  const contextValue = useMemo(
    () => ({
      ...state,
      isPending,
      signTransaction,
    }),
    [state, isPending, signTransaction],
  );

  return <WalletContext value={contextValue}>{children}</WalletContext>;
};
