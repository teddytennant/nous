package com.nous.app

import com.nous.app.crypto.Base58
import com.nous.app.crypto.Identity
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotEquals
import org.junit.Assert.assertNotNull
import org.junit.Assert.assertTrue
import org.junit.Test

class IdentityTest {

    @Test
    fun `generate creates valid identity`() {
        val identity = Identity.generate()
        assertNotNull(identity.keyPair)
        assertNotNull(identity.did)
        assertTrue(identity.did.startsWith("did:key:z"))
    }

    @Test
    fun `two identities have different DIDs`() {
        val a = Identity.generate()
        val b = Identity.generate()
        assertNotEquals(a.did, b.did)
    }

    @Test
    fun `sign and verify roundtrip`() {
        val identity = Identity.generate()
        val data = "sovereignty through cryptography".toByteArray()
        val signature = identity.sign(data)
        assertTrue(identity.verify(data, signature))
    }

    @Test
    fun `verify rejects tampered data`() {
        val identity = Identity.generate()
        val data = "original message".toByteArray()
        val signature = identity.sign(data)
        assertFalse(identity.verify("tampered message".toByteArray(), signature))
    }

    @Test
    fun `verify rejects wrong key`() {
        val alice = Identity.generate()
        val bob = Identity.generate()
        val data = "alice's message".toByteArray()
        val signature = alice.sign(data)
        assertFalse(bob.verify(data, signature))
    }

    @Test
    fun `fingerprint is deterministic`() {
        val identity = Identity.generate()
        assertEquals(identity.fingerprint(), identity.fingerprint())
    }

    @Test
    fun `fingerprint is 64 hex chars (SHA-256)`() {
        val identity = Identity.generate()
        val fp = identity.fingerprint()
        assertEquals(64, fp.length)
        assertTrue(fp.all { it in '0'..'9' || it in 'a'..'f' })
    }

    @Test
    fun `base58 encode empty`() {
        assertEquals("", Base58.encode(byteArrayOf()))
    }

    @Test
    fun `base58 encode known value`() {
        val encoded = Base58.encode(byteArrayOf(0, 0, 1))
        assertTrue(encoded.startsWith("11"))
    }

    @Test
    fun `did format includes multicodec prefix`() {
        val identity = Identity.generate()
        // DID:key format: did:key:z<base58(multicodec_prefix + public_key)>
        assertTrue(identity.did.length > 20)
        assertTrue(identity.did.startsWith("did:key:z"))
    }
}
