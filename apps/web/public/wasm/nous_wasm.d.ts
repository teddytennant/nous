/**
 * Type declarations for the nous-wasm module.
 *
 * These match the wasm-bindgen exports from crates/nous-wasm/src/lib.rs.
 * The stub JS file (nous_wasm.js) satisfies these types at compile time
 * but throws at runtime until the real WASM is built.
 */

/** Initialize the WASM module. */
export default function init(): Promise<void>;

/** Ed25519 + X25519 self-sovereign identity. */
export class WasmIdentity {
  constructor();
  static fromSigningKey(secretBytes: Uint8Array): WasmIdentity;
  readonly did: string;
  signingPublicKey(): Uint8Array;
  exchangePublicKey(): Uint8Array;
  exportSigningKey(): Uint8Array;
  sign(message: Uint8Array): Uint8Array;
  verify(message: Uint8Array, signature: Uint8Array): boolean;
  keyExchange(theirPublic: Uint8Array): Uint8Array;
  didDocument(): string;
}

/** AES-256-GCM encrypted payload. */
export class WasmEncrypted {
  readonly nonce: Uint8Array;
  readonly ciphertext: Uint8Array;
  toJson(): string;
  static fromJson(json: string): WasmEncrypted;
}

/** Schnorr proof of knowledge. */
export class WasmSchnorrProof {
  static prove(secret: Uint8Array, publicKey: Uint8Array, message: Uint8Array): WasmSchnorrProof;
  verify(publicKey: Uint8Array, message: Uint8Array): boolean;
  readonly commitment: Uint8Array;
  readonly response: Uint8Array;
}

/** Verify an Ed25519 signature given raw public key bytes, message, and signature. */
export function verifySignature(publicKey: Uint8Array, message: Uint8Array, signature: Uint8Array): boolean;

/** Encrypt plaintext with a 32-byte AES-256-GCM key. */
export function encrypt(key: Uint8Array, plaintext: Uint8Array): WasmEncrypted;

/** Decrypt ciphertext with a 32-byte AES-256-GCM key. */
export function decrypt(key: Uint8Array, encrypted: WasmEncrypted): Uint8Array;

/** Derive a 32-byte key from a shared secret and context using HKDF-SHA256. */
export function deriveKey(sharedSecret: Uint8Array, context: Uint8Array): Uint8Array;

/** SHA-256 hash of arbitrary data. */
export function sha256(data: Uint8Array): Uint8Array;

/** Convert a 32-byte Ed25519 public key to a DID:key string. */
export function publicKeyToDid(publicKey: Uint8Array): string;

/** Extract the 32-byte Ed25519 public key from a DID:key string. */
export function didToPublicKey(did: string): Uint8Array;

/** Generate a random Ristretto keypair (64 bytes: secret || public). */
export function schnorrKeygen(): Uint8Array;
