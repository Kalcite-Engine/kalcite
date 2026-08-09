# Unofficial NumWorks storage backend

NumWorks external applications do not receive a public EADK filesystem API. Kalcite therefore contains an explicitly unofficial storage adapter for Epsilon.

The adapter discovers Epsilon's storage through the live RAM metadata chain:

`SlotInfo -> UserlandHeader -> storage_address_ram/storage_size_ram`.

Before any read or write it validates the SlotInfo, UserlandHeader and filesystem magic values. The filesystem records are treated as bounded records containing a 16-bit record size, a NUL-terminated filename, and bytes. Delete compacts the following records; overwrite is delete + append. No heap allocation is used.

This architecture follows the publicly documented reverse-engineered layout used by NumWorks Extapp Storage / eadkp, but Kalcite's Rust implementation is independent and deliberately small. The original storage work by Yaya Cout is MIT-licensed and credited as the source of the reverse-engineered layout.

Because this is not an official EADK API, firmware changes can break it. `Storage.supported()` validates the runtime structures and returns false rather than writing if the layout is not recognized.

Kalcite's portable `Storage` API has the same semantics on desktop and NumWorks:

```klc
Storage.write_text("SAVE", "hello");
Storage.exists("SAVE");
Storage.size("SAVE");
Storage.checksum("SAVE");
Storage.remove("SAVE");
Storage.free_bytes();
Storage.total_bytes();
```

The native helper layer additionally exposes bounded byte reads/writes internally for libraries such as MessagePack.
