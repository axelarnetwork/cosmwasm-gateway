# Crypto Verify Contract

Cryptographic signature verification contract for CosmWasm chains without crypto API support (wasmd version <0.14.x).

ECDSA Secp256k1 parameters are currently supported.

## Formats

Input formats are serialized byte slices for Message, Signature, and Public Key.

### secp256k1:

- Message: A serialized message. It will be hashed by the contract using
  SHA-256, and the hashed value will be fed to the verification function.
- Signature: Serialized signature, in "compact" Cosmos format (64 bytes).
  Ethereum DER needs to be converted.
- Public Key: Compressed (33 bytes) or uncompressed (65 bytes) serialized public
  key, in SEC format.

Output is a boolean value indicating if verification succeeded or not.

## Remarks

In case of an error (wrong or unsupported inputs), the current implementation
returns an error, which can be easily handled by the contract, or returned to
the client.
