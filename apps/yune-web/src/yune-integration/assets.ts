/**
 * Explicit asset loader contract for yune-web
 *
 * Enforces that yune-web-owned default.yaml, schema YAML, and dictionary YAML
 * assets are explicitly provided and visible. No synthetic data is fabricated per D-06.
 *
 * Contract:
 * - Require explicit asset paths or content
 * - Fail visibly when assets are absent
 * - No synthetic/fake/substitute YAML content
 * - Assets are yune-web app-owned, not Yune-owned
 */

import { type YuneWebFilesystemAssets } from "@yune-ime/yune-web-runtime";

/**
 * Asset source types
 */
export type AssetSource =
  | { type: "content"; content: string | Uint8Array }
  | { type: "path"; path: string }
  | { type: "url"; url: string };

/**
 * Explicit yune-web assets configuration
 */
export interface ExplicitYuneWebAssets {
  defaultYaml: AssetSource;
  schemaYaml: AssetSource;
  dictionaryYaml: AssetSource;
  deployedDefaultYaml?: AssetSource;
  deployedSchemaYaml?: AssetSource;
}

/**
 * Load asset content from source
 *
 * @throws Error if asset is absent or loading fails
 */
export async function loadAssetContent(source: AssetSource): Promise<string | Uint8Array> {
  switch (source.type) {
    case "content":
      return source.content;

    case "path":
      // Note: File loading requires runtime environment (Node fs or fetch)
      // Implementation deferred to patch/application stage
      throw new Error(
        `Asset path loading not implemented: ${source.path}. ` +
          `Provide asset content explicitly or implement loader in patched worker.`,
      );

    case "url": {
      const response = await fetch(source.url);
      if (!response.ok) {
        throw new Error(`Asset URL loading failed: ${source.url} (${response.status})`);
      }
      if (isBinaryAssetUrl(source.url)) {
        return new Uint8Array(await response.arrayBuffer());
      }
      return response.text();
    }

    default:
      throw new Error(`Invalid asset source type: ${source}`);
  }
}

/**
 * Load explicit yune-web assets
 *
 * Converts ExplicitYuneWebAssets to YuneWebFilesystemAssets for
 * prepareYuneWebFilesystem.
 *
 * @throws Error if any asset is absent
 */
export async function loadExplicitAssets(
  assets: ExplicitYuneWebAssets,
): Promise<YuneWebFilesystemAssets> {
  const defaultYaml = await loadAssetContent(assets.defaultYaml);
  const schemaYaml = await loadAssetContent(assets.schemaYaml);
  const dictionaryYaml = await loadAssetContent(assets.dictionaryYaml);
  const deployedDefaultYaml =
    assets.deployedDefaultYaml === undefined ? undefined : await loadAssetContent(assets.deployedDefaultYaml);
  const deployedSchemaYaml =
    assets.deployedSchemaYaml === undefined ? undefined : await loadAssetContent(assets.deployedSchemaYaml);

  return {
    defaultYaml,
    schemaYaml,
    dictionaryYaml,
    deployedDefaultYaml,
    deployedSchemaYaml,
  };
}

/**
 * Validate that asset content is not fake/substitute data
 *
 * Rejects synthetic schema/dictionary patterns per D-06
 */
export function validateNoFallbackAssets(content: string | Uint8Array, assetName: string): void {
  if (typeof content !== "string") {
    // Binary content, skip string validation
    return;
  }

  if (content.trim().length === 0) {
    throw new Error(`yune-web asset ${assetName} is empty. Provide explicit app-owned YAML per D-06.`);
  }

  const forbiddenPatterns = [
    "synthetic yaml",
    "fake asset",
    "incomplete schema",
    "incomplete dictionary",
    "test yaml",
    "sample only",
    "TODO",
    "FIXME",
    "not available",
    "coming soon",
    "temporary yaml",
    "stub data",
  ];

  for (const pattern of forbiddenPatterns) {
    if (content.includes(pattern)) {
      throw new Error(
        `yune-web asset ${assetName} contains forbidden synthetic pattern: "${pattern}". ` +
          `Provide explicit app-owned YAML per D-06.`,
      );
    }
  }
}

/**
 * Validate explicit assets have no synthetic content
 *
 * @throws Error if any asset contains synthetic/fake patterns
 */
export function validateExplicitAssets(assets: YuneWebFilesystemAssets): void {
  validateNoFallbackAssets(assets.defaultYaml, "default.yaml");
  validateNoFallbackAssets(assets.schemaYaml, "schema YAML");
  validateNoFallbackAssets(assets.dictionaryYaml, "dictionary YAML");
  if (assets.deployedDefaultYaml !== undefined) {
    validateNoFallbackAssets(assets.deployedDefaultYaml, "deployed default.yaml");
  }
  if (assets.deployedSchemaYaml !== undefined) {
    validateNoFallbackAssets(assets.deployedSchemaYaml, "deployed schema YAML");
  }
}

function isBinaryAssetUrl(url: string): boolean {
  return /\.(?:bin|ocd2)(?:[?#].*)?$/i.test(url);
}

/**
 * Asset requirement checklist for yune-web
 *
 * Documents required assets and their sources before runtime init
 */
export interface AssetRequirementChecklist {
  schemaId: string;
  dictionaryId: string;
  assets: ExplicitYuneWebAssets;
  validated: boolean;
}

/**
 * Create asset requirement checklist
 *
 * Records required assets before init, enforces explicit sources
 */
export function createAssetChecklist(
  schemaId: string,
  dictionaryId: string,
  assets: ExplicitYuneWebAssets,
): AssetRequirementChecklist {
  return {
    schemaId,
    dictionaryId,
    assets,
    validated: false,
  };
}

/**
 * Verify asset checklist is complete before runtime init
 *
 * @throws Error if assets are not validated
 */
export function verifyAssetChecklist(checklist: AssetRequirementChecklist): void {
  if (!checklist.validated) {
    throw new Error(
      `Asset checklist for schema ${checklist.schemaId} not validated. ` +
        `Call loadExplicitAssets and validateExplicitAssets before init.`,
    );
  }
}
