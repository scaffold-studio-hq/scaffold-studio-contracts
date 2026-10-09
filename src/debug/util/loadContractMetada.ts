/* eslint-disable @typescript-eslint/no-unsafe-member-access */
/* eslint-disable @typescript-eslint/no-unsafe-argument */
/* eslint-disable @typescript-eslint/no-unsafe-assignment */
import { Server } from "@stellar/stellar-sdk/rpc";
import { network } from "../../contracts/util";
import { Contract } from "@stellar/stellar-sdk";
import { getWasmContractData } from "./getWasmContractData";
import {
  CONTRACT_SECTIONS,
  ContractData,
  ContractSectionName,
} from "../types/types";
import { getWasmContractData as decodeWasmContractData } from "./getWasmContractData";

export interface ContractMetadata {
  contractmetav0?: { [key: string]: string };
  contractenvmetav0?: { [key: string]: string };
  wasmHash?: string;
  wasmBinary?: string;
}

export const loadContractMetadata = async (contractId: string) => {
  try {
    const wasmHash = await loadWasmHash(contractId);
    if (!wasmHash) {
      throw new Error(`Failed to load WASM hash for contract ${contractId}`);
    }
    const wasm = await loadWasmBinary(wasmHash);
    if (!wasm) {
      throw new Error(`Failed to load WASM binary for hash ${wasmHash}`);
    }

    const wasmData = await getWasmContractData(wasm);

    const metadata = {
      contractmetav0:
        wasmData && wasmData.contractmetav0
          ? (wasmData.contractmetav0 as unknown)
          : undefined,

      contractenvmetav0:
        wasmData && wasmData.contractenvmetav0
          ? (wasmData.contractenvmetav0 as unknown)
          : undefined,
      wasmHash,
      wasmBinary: wasm.toString("hex"),
    };

    return { ...metadata } as ContractMetadata;
  } catch (error) {
    console.error(`Failed to load contract metadata for ${contractId}:`, error);
    return {};
  }
};

const loadWasmHash = async (contractId: string) => {
  try {
    const server = new Server(network.rpcUrl, { allowHttp: true });

    const contractLedgerKey = new Contract(contractId).getFootprint();
    const response = await server.getLedgerEntries(contractLedgerKey);
    if (!response.entries.length || !response.entries[0]?.val) {
      throw new Error(`No entries found for contract ${contractId}`);
    }
    const wasmHash = response.entries[0].val
      .contractData()
      .val()
      .instance()
      .executable()
      .wasmHash()
      .toString("hex");

    return wasmHash;
  } catch (error) {
    console.error(`Failed to load contract metadata for ${contractId}:`, error);
    return null;
  }
};

const loadWasmBinary = async (wasmHash: string) => {
  try {
    const server = new Server(network.rpcUrl, { allowHttp: true });

    return await server.getContractWasmByHash(wasmHash, "hex");
  } catch (error) {
    console.error(`Failed to load contract metadata for ${wasmHash}:`, error);
    return null;
  }
};

/**
 * Present decoded section entries in the metadata shape consumed by the Debugger.
 * The WASM/XDR decoding itself is shared with getWasmContractData.ts.
 */
export const getWasmContractData = async (wasmBytes: Buffer) => {
  try {
    const decodedSections = await decodeWasmContractData(wasmBytes);
    if (!decodedSections) return null;

    const result: Record<ContractSectionName, ContractData> = {
      contractmetav0: {},
      contractenvmetav0: {},
      contractspecv0: {},
    };

    for (const sectionName of CONTRACT_SECTIONS) {
      for (const json of decodedSections[sectionName].json ?? []) {
        const sectionDataJson = JSON.parse(json) as Record<string, unknown>;
        const sectionContent: Record<string, unknown> = {};

        for (const entry of Object.values(sectionDataJson)) {
          if (!entry || typeof entry !== "object") continue;
          const values = entry as Record<string, unknown>;
          if (values.key) {
            sectionContent[String(values.key)] = values.val;
          } else {
            Object.assign(sectionContent, values);
          }
        }

        result[sectionName] = { ...result[sectionName], ...sectionContent };
      }
    }
    return result;
    // eslint-disable-next-line @typescript-eslint/no-unused-vars
  } catch (e) {
    return null;
  }
};
