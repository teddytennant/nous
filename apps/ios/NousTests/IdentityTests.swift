import Foundation
import Testing
@testable import Nous

@Test func identityGeneration() {
    let identity = NousIdentity()
    #expect(!identity.did.isEmpty)
    #expect(identity.did.hasPrefix("did:key:z"))
}

@Test func twoIdentitiesAreDifferent() {
    let a = NousIdentity()
    let b = NousIdentity()
    #expect(a.did != b.did)
}

@Test func signAndVerify() throws {
    let identity = NousIdentity()
    let data = Data("sovereignty through cryptography".utf8)
    let signature = try identity.sign(data)
    #expect(identity.verify(signature: signature, for: data))
}

@Test func verifyRejectsTamperedData() throws {
    let identity = NousIdentity()
    let data = Data("original".utf8)
    let signature = try identity.sign(data)
    let tampered = Data("tampered".utf8)
    #expect(!identity.verify(signature: signature, for: tampered))
}

@Test func verifyRejectsWrongKey() throws {
    let alice = NousIdentity()
    let bob = NousIdentity()
    let data = Data("alice's message".utf8)
    let signature = try alice.sign(data)
    #expect(!bob.verify(signature: signature, for: data))
}

@Test func fingerprintIsDeterministic() {
    let identity = NousIdentity()
    #expect(identity.fingerprint == identity.fingerprint)
}

@Test func fingerprintIs64HexChars() {
    let identity = NousIdentity()
    let fp = identity.fingerprint
    #expect(fp.count == 64)
    #expect(fp.allSatisfy { $0.isHexDigit })
}

@Test func restoreFromPrivateKey() throws {
    let original = NousIdentity()
    let exported = original.rawPrivateKey
    let restored = try NousIdentity(rawPrivateKey: exported)
    #expect(original.did == restored.did)
}

@Test func signatureConsistentAfterRestore() throws {
    let original = NousIdentity()
    let data = Data("test".utf8)
    let sig = try original.sign(data)

    let restored = try NousIdentity(rawPrivateKey: original.rawPrivateKey)
    #expect(restored.verify(signature: sig, for: data))
}

@Test func staticVerification() throws {
    let identity = NousIdentity()
    let data = Data("static verify".utf8)
    let sig = try identity.sign(data)
    let valid = try NousIdentity.verify(
        signature: sig,
        for: data,
        publicKey: identity.publicKeyBytes
    )
    #expect(valid)
}

@Test func base58EncodeEmpty() {
    #expect(Base58.encode(Data()) == "")
}

@Test func base58EncodeDeterministic() {
    let data = Data([1, 2, 3])
    #expect(Base58.encode(data) == Base58.encode(data))
}

@Test func didFormatIncludesMulticodecPrefix() {
    let identity = NousIdentity()
    #expect(identity.did.count > 20)
    #expect(identity.did.hasPrefix("did:key:z"))
}
