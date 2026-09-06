# STDF Fixture Corpus Policy

This directory stores small, redistributable STDF fixtures as whitespace-separated
hex bytes. The goal is to keep coverage source-controlled without committing
vendor-confidential production files.

## Fixture Rules

- Fixtures must be synthetic, public-domain, or explicitly cleared for commit.
- Binary STDF files should be stored as `.hex` unless raw bytes are required.
- Each fixture should be small enough for quick unit tests.
- Real vendor files should not be committed unless their provenance is documented
  here and the file is safe to redistribute.

## Golden Fixtures

- `minimal_le.hex`: minimal little-endian FAR-only STDF.
- `minimal_be.hex`: minimal big-endian FAR-only STDF.
- `ptr_prr_flow.hex`: little-endian FAR -> PIR -> PTR -> PRR flow.
- `malformed_truncated.hex`: valid FAR followed by a truncated PTR frame.
- `unknown_vendor.hex`: valid FAR followed by an unknown vendor-extension record.

## Real Vendor Corpus Policy

For private vendor data, keep the files outside the repository and add only:

- a sanitized fixture name
- source/vendor family if allowed
- STDF record families covered
- expected row/record counts
- whether redistribution is permitted

The test suite should support adding those files locally without requiring them
for public CI.
