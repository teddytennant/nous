/**
 * Nous client-side crypto via WebAssembly.
 *
 * The WASM module (nous-wasm) provides ed25519 identity, AES-256-GCM encryption,
 * X25519 key exchange, Schnorr ZK proofs, and DID:key utilities — all running
 * locally in the browser. No private keys leave the client.
 *
 * Usage:
 *   import { initCrypto, getIdentity, signMessage } from "@/lib/crypto";
 *   await initCrypto();                  // loads WASM (once)
 *   const id = getIdentity();           // returns current WasmIdentity or null
 *   const sig = await signMessage(msg); // signs with local key
 */

// Types matching the WASM exports
export interface NousIdentity {
  did: string;
  signingPublicKey: Uint8Array;
  exchangePublicKey: Uint8Array;
  sign: (message: Uint8Array) => Uint8Array;
  verify: (message: Uint8Array, signature: Uint8Array) => boolean;
  keyExchange: (theirPublicKey: Uint8Array) => Uint8Array;
  exportSigningKey: () => Uint8Array;
  didDocument: () => string;
}

export interface CryptoModule {
  WasmIdentity: {
    new: () => NousIdentity;
    fromSigningKey: (key: Uint8Array) => NousIdentity;
  };
  encrypt: (key: Uint8Array, plaintext: Uint8Array) => { nonce: Uint8Array; ciphertext: Uint8Array; toJson: () => string };
  decrypt: (key: Uint8Array, encrypted: { nonce: Uint8Array; ciphertext: Uint8Array }) => Uint8Array;
  deriveKey: (sharedSecret: Uint8Array, context: Uint8Array) => Uint8Array;
  sha256: (data: Uint8Array) => Uint8Array;
  verifySignature: (publicKey: Uint8Array, message: Uint8Array, signature: Uint8Array) => boolean;
  publicKeyToDid: (publicKey: Uint8Array) => string;
  didToPublicKey: (did: string) => Uint8Array;
}

let wasmModule: CryptoModule | null = null;
let currentIdentity: NousIdentity | null = null;
let initPromise: Promise<void> | null = null;

const SIGNING_KEY_STORAGE = "nous_signing_key";

/**
 * Initialize the WASM crypto module. Idempotent — safe to call multiple times.
 * Returns true if WASM loaded successfully, false otherwise.
 */
export async function initCrypto(): Promise<boolean> {
  if (wasmModule) return true;

  if (initPromise) {
    await initPromise;
    return wasmModule !== null;
  }

  initPromise = (async () => {
    try {
      // Dynamic import — the WASM package must be built with wasm-pack
      // and placed at apps/web/public/wasm/nous_wasm.js (or via npm package)
      const mod = await import("../../public/wasm/nous_wasm.js");
      if (mod.default) await mod.default(); // init WASM
      wasmModule = mod as unknown as CryptoModule;

      // Try to restore identity from localStorage
      const stored = localStorage.getItem(SIGNING_KEY_STORAGE);
      if (stored && wasmModule) {
        try {
          const keyBytes = Uint8Array.from(atob(stored), (c) => c.charCodeAt(0));
          currentIdentity = wasmModule.WasmIdentity.fromSigningKey(keyBytes);
        } catch {
          // Stored key is invalid — clear it
          localStorage.removeItem(SIGNING_KEY_STORAGE);
        }
      }
    } catch {
      // WASM not available (SSR, missing build, etc.) — continue without it
      wasmModule = null;
    }
  })();

  await initPromise;
  return wasmModule !== null;
}

/**
 * Check if WASM crypto is available.
 */
export function isCryptoAvailable(): boolean {
  return wasmModule !== null;
}

/**
 * Generate a new identity. Stores the signing key in localStorage.
 * Returns the DID string.
 */
export function generateIdentity(): string | null {
  if (!wasmModule) return null;
  currentIdentity = wasmModule.WasmIdentity.new();
  const keyBytes = currentIdentity.exportSigningKey();
  const keyB64 = btoa(String.fromCharCode(...keyBytes));
  localStorage.setItem(SIGNING_KEY_STORAGE, keyB64);
  localStorage.setItem("nous_did", currentIdentity.did);
  return currentIdentity.did;
}

/**
 * Get the current local identity, if one exists.
 */
export function getIdentity(): NousIdentity | null {
  return currentIdentity;
}

/**
 * Get the current DID string.
 */
export function getDid(): string | null {
  return currentIdentity?.did ?? localStorage.getItem("nous_did");
}

/**
 * Sign a message with the local identity.
 * Returns base64-encoded signature, or null if no identity.
 */
export function signMessage(message: string): string | null {
  if (!currentIdentity) return null;
  const msgBytes = new TextEncoder().encode(message);
  const sig = currentIdentity.sign(msgBytes);
  return btoa(String.fromCharCode(...sig));
}

/**
 * Sign arbitrary bytes with the local identity.
 */
export function signBytes(data: Uint8Array): Uint8Array | null {
  if (!currentIdentity) return null;
  return currentIdentity.sign(data);
}

/**
 * Verify a signature against a public key (DID).
 */
export function verifySignature(
  did: string,
  message: Uint8Array,
  signature: Uint8Array
): boolean {
  if (!wasmModule) return false;
  try {
    const pubkey = wasmModule.didToPublicKey(did);
    return wasmModule.verifySignature(pubkey, message, signature);
  } catch {
    return false;
  }
}

/**
 * Encrypt data with a symmetric key.
 */
export function encrypt(
  key: Uint8Array,
  plaintext: Uint8Array
): { nonce: Uint8Array; ciphertext: Uint8Array } | null {
  if (!wasmModule) return null;
  return wasmModule.encrypt(key, plaintext);
}

/**
 * Decrypt data with a symmetric key.
 */
export function decrypt(
  key: Uint8Array,
  nonce: Uint8Array,
  ciphertext: Uint8Array
): Uint8Array | null {
  if (!wasmModule) return null;
  return wasmModule.decrypt(key, { nonce, ciphertext });
}

/**
 * Derive a symmetric key from a shared secret and context.
 */
export function deriveKey(sharedSecret: Uint8Array, context: string): Uint8Array | null {
  if (!wasmModule) return null;
  const contextBytes = new TextEncoder().encode(context);
  return wasmModule.deriveKey(sharedSecret, contextBytes);
}

/**
 * Perform X25519 key exchange with a peer's public key.
 * Returns a 32-byte shared secret.
 */
export function keyExchange(theirPublicKey: Uint8Array): Uint8Array | null {
  if (!currentIdentity) return null;
  return currentIdentity.keyExchange(theirPublicKey);
}

/**
 * Hash data with SHA-256.
 */
export function sha256(data: Uint8Array): Uint8Array | null {
  if (!wasmModule) return null;
  return wasmModule.sha256(data);
}

/**
 * Get the DID Document as a JSON object.
 */
export function getDidDocument(): Record<string, unknown> | null {
  if (!currentIdentity) return null;
  try {
    return JSON.parse(currentIdentity.didDocument());
  } catch {
    return null;
  }
}

/**
 * Delete the local identity and clear stored keys.
 */
export function clearIdentity(): void {
  currentIdentity = null;
  localStorage.removeItem(SIGNING_KEY_STORAGE);
}
