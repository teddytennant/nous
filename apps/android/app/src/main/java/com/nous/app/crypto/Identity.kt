package com.nous.app.crypto

import java.security.KeyPair
import java.security.KeyPairGenerator
import java.security.MessageDigest
import java.security.SecureRandom
import java.security.Signature

/**
 * Self-sovereign identity based on Ed25519 keys and DID:key method.
 */
class Identity private constructor(
    val keyPair: KeyPair,
    val did: String,
) {
    companion object {
        /**
         * Generate a new Ed25519 identity.
         */
        fun generate(): Identity {
            val keyGen = KeyPairGenerator.getInstance("Ed25519")
            keyGen.initialize(255, SecureRandom())
            val keyPair = keyGen.generateKeyPair()
            val did = publicKeyToDid(keyPair.public.encoded)
            return Identity(keyPair, did)
        }

        /**
         * Convert a public key to a DID:key identifier.
         * Uses the multicodec prefix 0xed01 for Ed25519.
         */
        fun publicKeyToDid(publicKeyBytes: ByteArray): String {
            // Ed25519 multicodec prefix
            val multicodecPrefix = byteArrayOf(0xed.toByte(), 0x01)
            val prefixed = multicodecPrefix + publicKeyBytes
            val encoded = Base58.encode(prefixed)
            return "did:key:z$encoded"
        }
    }

    /**
     * Sign arbitrary data with this identity's private key.
     */
    fun sign(data: ByteArray): ByteArray {
        val signer = Signature.getInstance("Ed25519")
        signer.initSign(keyPair.private)
        signer.update(data)
        return signer.sign()
    }

    /**
     * Verify a signature against this identity's public key.
     */
    fun verify(data: ByteArray, signature: ByteArray): Boolean {
        val verifier = Signature.getInstance("Ed25519")
        verifier.initVerify(keyPair.public)
        verifier.update(data)
        return verifier.verify(signature)
    }

    /**
     * Get the SHA-256 fingerprint of the public key.
     */
    fun fingerprint(): String {
        val digest = MessageDigest.getInstance("SHA-256")
        val hash = digest.digest(keyPair.public.encoded)
        return hash.joinToString("") { "%02x".format(it) }
    }
}

/**
 * Minimal Base58 encoder (Bitcoin alphabet).
 */
object Base58 {
    private const val ALPHABET = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz"

    fun encode(input: ByteArray): String {
        if (input.isEmpty()) return ""

        // Count leading zeros
        var zeros = 0
        while (zeros < input.size && input[zeros] == 0.toByte()) zeros++

        // Convert to base58
        val encoded = CharArray(input.size * 2)
        var outputStart = encoded.size
        var inputStart = zeros

        while (inputStart < input.size) {
            outputStart--
            encoded[outputStart] = ALPHABET[divmod(input, inputStart, 256, 58).toInt()]
            if (input[inputStart] == 0.toByte()) inputStart++
        }

        while (outputStart < encoded.size && encoded[outputStart] == ALPHABET[0]) outputStart++
        repeat(zeros) { outputStart--; encoded[outputStart] = '1' }

        return String(encoded, outputStart, encoded.size - outputStart)
    }

    private fun divmod(number: ByteArray, firstDigit: Int, base: Int, divisor: Int): Byte {
        var remainder = 0
        for (i in firstDigit until number.size) {
            val digit = number[i].toInt() and 0xFF
            val temp = remainder * base + digit
            number[i] = (temp / divisor).toByte()
            remainder = temp % divisor
        }
        return remainder.toByte()
    }
}
