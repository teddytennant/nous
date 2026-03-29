import CryptoKit
import Foundation

/// Self-sovereign identity based on Ed25519 and the DID:key method.
///
/// Each identity owns an Ed25519 keypair and derives a W3C-compliant
/// decentralized identifier (DID) from the public key.
public struct NousIdentity: Sendable {
    /// The Ed25519 signing key pair.
    public let privateKey: Curve25519.Signing.PrivateKey
    /// The DID:key identifier.
    public let did: String
    /// The raw public key bytes.
    public var publicKeyBytes: Data {
        privateKey.publicKey.rawRepresentation
    }

    /// Generate a new random identity.
    public init() {
        self.privateKey = Curve25519.Signing.PrivateKey()
        self.did = Self.publicKeyToDID(privateKey.publicKey.rawRepresentation)
    }

    /// Restore an identity from a raw private key.
    public init(rawPrivateKey: Data) throws {
        self.privateKey = try Curve25519.Signing.PrivateKey(rawRepresentation: rawPrivateKey)
        self.did = Self.publicKeyToDID(privateKey.publicKey.rawRepresentation)
    }

    /// Sign arbitrary data.
    public func sign(_ data: Data) throws -> Data {
        try privateKey.signature(for: data)
    }

    /// Verify a signature against this identity's public key.
    public func verify(signature: Data, for data: Data) -> Bool {
        privateKey.publicKey.isValidSignature(signature, for: data)
    }

    /// Verify a signature against an arbitrary public key.
    public static func verify(
        signature: Data,
        for data: Data,
        publicKey: Data
    ) throws -> Bool {
        let key = try Curve25519.Signing.PublicKey(rawRepresentation: publicKey)
        return key.isValidSignature(signature, for: data)
    }

    /// SHA-256 fingerprint of the public key.
    public var fingerprint: String {
        let hash = SHA256.hash(data: publicKeyBytes)
        return hash.map { String(format: "%02x", $0) }.joined()
    }

    /// Export the private key bytes (handle with care — zeroize after use).
    public var rawPrivateKey: Data {
        privateKey.rawRepresentation
    }

    // MARK: - DID:key

    /// Convert a public key to a DID:key identifier.
    /// Uses multicodec prefix 0xed01 for Ed25519.
    static func publicKeyToDID(_ publicKey: Data) -> String {
        var prefixed = Data([0xED, 0x01])
        prefixed.append(publicKey)
        let encoded = Base58.encode(prefixed)
        return "did:key:z\(encoded)"
    }
}

/// Minimal Base58 encoder (Bitcoin alphabet).
public enum Base58 {
    private static let alphabet = Array("123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz")

    public static func encode(_ data: Data) -> String {
        guard !data.isEmpty else { return "" }

        var bytes = Array(data)
        var zeros = 0
        while zeros < bytes.count && bytes[zeros] == 0 { zeros += 1 }

        var result: [Character] = []
        var start = zeros

        while start < bytes.count {
            var carry = 0
            for i in start..<bytes.count {
                carry = carry * 256 + Int(bytes[i])
                bytes[i] = UInt8(carry / 58)
                carry %= 58
            }
            result.append(alphabet[carry])
            while start < bytes.count && bytes[start] == 0 { start += 1 }
        }

        let prefix = String(repeating: "1", count: zeros)
        return prefix + String(result.reversed())
    }
}
