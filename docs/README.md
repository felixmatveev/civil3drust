# Format findings

One file per finding or object type (e.g. `alignment-object.md`). Each
finding should record:

1. **Sample files** used (store small ones in `samples/`, describe large ones).
2. **How they were made** — Civil 3D version, template, exact steps.
3. **What differed** between samples, and the bytes or fields that changed.
4. **Conclusion** and confidence (confirmed by a round trip in Civil 3D, or
   inferred only).

Never record information taken from Autodesk SDKs, headers, or decompiled
binaries. See the cleanroom rules in the top-level README.
