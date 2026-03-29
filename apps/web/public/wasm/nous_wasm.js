/**
 * Stub module for nous-wasm.
 *
 * This file exists so the web app can compile and pass type-checking without
 * having the actual WASM build artifacts present.  Every exported function
 * throws an error directing the developer to build the real WASM module first:
 *
 *   cd crates/nous-wasm && wasm-pack build --target web --out-dir ../../apps/web/public/wasm
 */

const STUB_ERROR =
  "nous-wasm has not been built. Run: wasm-pack build --target web " +
  "in crates/nous-wasm to generate the real module.";

function stubError() {
  throw new Error(STUB_ERROR);
}

// ── Init (default export expected by wasm-pack output) ───────────

export default function init() {
  // no-op — the real init() loads the .wasm binary
  return Promise.resolve();
}

// ── WasmIdentity ─────────────────────────────────────────────────

export class WasmIdentity {
  constructor() {
    stubError();
  }

  static fromSigningKey(_secretBytes) {
    stubError();
  }

  get did() {
    stubError();
  }

  signingPublicKey() {
    stubError();
  }

  exchangePublicKey() {
    stubError();
  }

  exportSigningKey() {
    stubError();
  }

  sign(_message) {
    stubError();
  }

  verify(_message, _signature) {
    stubError();
  }

  keyExchange(_theirPublic) {
    stubError();
  }

  didDocument() {
    stubError();
  }
}

// ── WasmEncrypted ────────────────────────────────────────────────

export class WasmEncrypted {
  get nonce() {
    stubError();
  }

  get ciphertext() {
    stubError();
  }

  toJson() {
    stubError();
  }

  static fromJson(_json) {
    stubError();
  }
}

// ── WasmSchnorrProof ─────────────────────────────────────────────

export class WasmSchnorrProof {
  static prove(_secret, _public, _message) {
    stubError();
  }

  verify(_public, _message) {
    stubError();
  }

  get commitment() {
    stubError();
  }

  get response() {
    stubError();
  }
}

// ── Standalone functions ─────────────────────────────────────────

export function verifySignature(_publicKey, _message, _signature) {
  stubError();
}

export function encrypt(_key, _plaintext) {
  stubError();
}

export function decrypt(_key, _encrypted) {
  stubError();
}

export function deriveKey(_sharedSecret, _context) {
  stubError();
}

export function sha256(_data) {
  stubError();
}

export function publicKeyToDid(_publicKey) {
  stubError();
}

export function didToPublicKey(_did) {
  stubError();
}

export function schnorrKeygen() {
  stubError();
}
