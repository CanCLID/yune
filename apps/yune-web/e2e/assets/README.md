# TypeDuck-Web Browser E2E Assets

This directory contains TypeDuck-Web-owned YAML assets required for browser E2E validation.

## Required Assets Per D-06

Browser E2E MUST use explicit TypeDuck-Web-owned YAML files. Generating substitute, synthetic, test-only, or fabricated schema/dictionary data is FORBIDDEN.

### Asset List

Before running browser E2E, provide these assets:

1. **default.yaml** — RIME default configuration
2. **schema YAML** — Schema definition file (e.g., `luna_pinyin.schema.yaml`)
3. **dictionary YAML** — Dictionary source file (e.g., `luna_pinyin.dict.yaml`)

### Asset Sources

Assets MUST come from TypeDuck-Web or its documented upstream source:

- Tracked yune-web schema assets: `apps/yune-web/public/schema/`
- Historical TypeDuck-Web reference checkout when present locally
- TypeDuck-Web CDN URLs documented in upstream README
- Upstream RIME project asset repositories
- User-provided real schema/dictionary YAML files

### Asset Validation

Assets are validated by `@yune-ime/typeduck-runtime` filesystem helpers:

```typescript
import { validateExplicitAssets } from "@yune-ime/typeduck-runtime";

validateExplicitAssets({
  defaultYaml: /* loaded asset content */,
  schemaYaml: /* loaded asset content */,
  dictionaryYaml: /* loaded asset content */,
});
```

Validation rejects:

- Empty or missing content
- Synthetic test-only data (not from TypeDuck-Web)
- Synthetic test-only data
- Fabricated substitute schema or dictionary content

### Copy/Reference Instructions

**Option 1: Copy from tracked yune-web assets**

```bash
cp apps/yune-web/public/schema/default.yaml apps/yune-web/e2e/assets/
cp apps/yune-web/public/schema/*.schema.yaml apps/yune-web/e2e/assets/
cp apps/yune-web/public/schema/*.dict.yaml apps/yune-web/e2e/assets/
```

**Option 2: Reference in test configuration**

```typescript
// In E2E test setup
const assetsPath = path.resolve("../public/schema");
const defaultYaml = fs.readFileSync(path.join(assetsPath, "default.yaml"), "utf8");
const schemaYaml = fs.readFileSync(path.join(assetsPath, "luna_pinyin.schema.yaml"), "utf8");
const dictionaryYaml = fs.readFileSync(path.join(assetsPath, "luna_pinyin.dict.yaml"), "utf8");
```

**Option 3: Fetch from documented CDN**

```bash
# If TypeDuck-Web documents asset CDN URLs
curl -o default.yaml <TypeDuck-Web-CDN-URL>/default.yaml
curl -o luna_pinyin.schema.yaml <TypeDuck-Web-CDN-URL>/luna_pinyin.schema.yaml
curl -o luna_pinyin.dict.yaml <TypeDuck-Web-CDN-URL>/luna_pinyin.dict.yaml
```

### Forbidden Patterns

E2E assets directory MUST NOT contain:

- Substitute schema files (not from TypeDuck-Web)
- Substitute dictionary files (not from TypeDuck-Web)
- Test-only fabricated schema files
- Test-only fabricated dictionary files
- Empty YAML files
- YAML files with `TODO`/`FIXME` content

Grep-gate verification:

```bash
# Verify no forbidden asset files exist
FORBIDDEN="fallback.schema.yaml fallback.dict.yaml dummy.schema.yaml dummy.dict.yaml"
for pattern in $FORBIDDEN; do
  if grep -r "$pattern" apps/yune-web/e2e/assets --include="*.yaml"; then
    echo "VIOLATION: Found forbidden file $pattern"
    exit 1
  fi
done
echo "PASSED: No forbidden substitute files found"
```

## Asset Loading in E2E

Browser E2E tests MUST load assets explicitly before runtime initialization:

```typescript
import { prepareTypeDuckFilesystem } from "@yune-ime/typeduck-runtime";

const assets = {
  defaultYaml: await loadAsset("default.yaml"),
  schemaYaml: await loadAsset("luna_pinyin.schema.yaml"),
  dictionaryYaml: await loadAsset("luna_pinyin.dict.yaml"),
};

prepareTypeDuckFilesystem(Module.FS, {
  sharedDataDir: "/typeduck/shared",
  userDataDir: "/typeduck/user",
  schemaId: "luna_pinyin",
  dictionaryId: "luna_pinyin",
  assets,
});
```

Missing assets MUST fail visibly at filesystem preparation. Tests MUST NOT proceed with fabricated fallback data.

## Evidence

Asset loading evidence MUST be recorded in `e2e/results/`:

- `asset-sources.log` — Documented source paths/URLs for each asset
- `asset-validation.log` — Validation output from runtime helpers
- `asset-load-error.log` — Missing asset failures (if any)

---

**Phase**: 10-typeduck-web-app-integration-and-e2e
**Plan**: 10-03 (Real browser E2E/smoke validation)
**Requirement**: TYPEDUCK-E2E-03, D-06
**Status**: Asset scaffolding with explicit TypeDuck-Web-owned YAML requirement
