import { afterEach, describe, expect, it, vi } from "vitest";

import { loadAssetContent, loadExplicitAssets, validateExplicitAssets } from "./assets.js";

describe("yune-web explicit asset loading", () => {
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("loads app-owned YAML from browser URLs", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn(async () => new Response("schema:\n  schema_id: luna_pinyin\n", { status: 200 })),
    );

    await expect(loadAssetContent({ type: "url", url: "schema/luna_pinyin.schema.yaml" })).resolves.toBe(
      "schema:\n  schema_id: luna_pinyin\n",
    );
  });

  it("loads optional deployed build YAML when the app provides it", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn(async (url: string) => new Response(`content:${url}`, { status: 200 })),
    );

    await expect(
      loadExplicitAssets({
        defaultYaml: { type: "url", url: "schema/default.yaml" },
        schemaYaml: { type: "url", url: "schema/jyut6ping3_mobile.schema.yaml" },
        dictionaryYaml: { type: "url", url: "schema/jyut6ping3.dict.yaml" },
        deployedDefaultYaml: { type: "url", url: "schema/build/default.yaml" },
        deployedSchemaYaml: { type: "url", url: "schema/build/jyut6ping3_mobile.schema.yaml" },
      }),
    ).resolves.toMatchObject({
      defaultYaml: "content:schema/default.yaml",
      schemaYaml: "content:schema/jyut6ping3_mobile.schema.yaml",
      dictionaryYaml: "content:schema/jyut6ping3.dict.yaml",
      deployedDefaultYaml: "content:schema/build/default.yaml",
      deployedSchemaYaml: "content:schema/build/jyut6ping3_mobile.schema.yaml",
    });
  });

  it("surfaces missing URL assets as clear init blockers", async () => {
    vi.stubGlobal("fetch", vi.fn(async () => new Response("missing", { status: 404 })));

    await expect(loadAssetContent({ type: "url", url: "schema/default.yaml" })).rejects.toThrow(
      "Asset URL loading failed: schema/default.yaml (404)",
    );
  });

  it("rejects empty explicit YAML instead of fabricating fallback assets", () => {
    expect(() =>
      validateExplicitAssets({
        defaultYaml: "",
        schemaYaml: "schema:\n  schema_id: luna_pinyin\n",
        dictionaryYaml: "---\nname: luna_pinyin\n...\n",
      }),
    ).toThrow("yune-web asset default.yaml is empty");
  });
});
