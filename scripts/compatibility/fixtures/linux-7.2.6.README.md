# Retained authenticated small source object

`linux-7.2.6.dsc` is the exact 7,942-byte source descriptor retained by
`halcyon-021-evidence-recovery/native-input-closure-r1` (read-only reference).
SHA-256: `fe551a6a65e6a2d288d77a9528f7d330b16452be473eeb6b17ffc804f8d27bed`.

The existing `multipocket/0-InRelease` and `2-Sources.gz` authenticate its record
in the Pop noble archive. The test checks the real Release signature, index
size/hash, and this descriptor's signed size/hash, then exercises both streaming
HTTPS (a local response fixture; no request) and retained adapters. No additional
trust key or signed metadata was generated. This is not a kernel source closure
or native qualification; the corresponding 236,700,068-byte tar was not fetched.

Original record URL:
https://apt.pop-os.org/release/pool/noble/linux/141c6f50e8024ba2c6e7ee9276d7c1c703f6b428/linux_7.2.6-76070206.202609141300~1792433904~24.04~141c6f5.dsc
