import { describe, expect, it } from "vitest";

import {
  WEB06_ACTION_OWNER,
  web06LiveOptionFanout,
  web06SchemaChangeFanout,
} from "../src/yune-integration/web06-app-action-map.js";

const deployPreferences = {
  pageSize: 6,
  enableCompletion: true,
  enableCorrection: false,
  enableSentence: true,
  enableLearning: true,
  combineCandidates: true,
  predictionNeverFirst: true,
  predictionThreshold: 0,
  dictionaryExclude: [],
  isCangjie5: true,
};

const liveOptions = {
  isAsciiMode: false,
  isFullShape: false,
  outputStandard: "hong_kong_traditional" as const,
  activeSchema: "jyut6ping3" as const,
  isExtendedCharset: false,
  isDisabled: false,
};

describe("WEB-06 app action fanouts", () => {
  it("freezes the exact 12-action live option effect", () => {
    const fanout = web06LiveOptionFanout(liveOptions);
    expect(fanout).toHaveLength(12);
    expect(fanout.every(item => item.owner === WEB06_ACTION_OWNER.liveOptions)).toBe(true);
    expect(fanout.map(item => item.action.args)).toEqual([
      ["soft_cursor", true],
      ["ascii_mode", false],
      ["full_shape", false],
      ["traditionalization", false],
      ["variants_hk", true],
      ["trad_tw", false],
      ["simplification", false],
      ["zh_hans", false],
      ["zh_hant_hk", false],
      ["zh_hant_tw", false],
      ["extended_charset", false],
      ["disabled", false],
    ]);
  });

  it("freezes actual cross-effect schema enqueue order", () => {
    const fanout = web06SchemaChangeFanout({
      nextSchema: "luna_pinyin",
      deployPreferences,
      liveOptions,
      applyDeployPreferences: true,
    });
    expect(fanout.map(item => [item.owner, item.action.name, item.action.args])).toEqual([
      [WEB06_ACTION_OWNER.schema, "selectSchema", ["luna_pinyin"]],
      [WEB06_ACTION_OWNER.deployPreferences, "customize", [deployPreferences]],
      [WEB06_ACTION_OWNER.liveOptions, "setOption", ["soft_cursor", true]],
      [WEB06_ACTION_OWNER.deployPreferences, "deploy", []],
      ...web06LiveOptionFanout({ ...liveOptions, activeSchema: "luna_pinyin" })
        .slice(1)
        .map(item => [item.owner, item.action.name, item.action.args]),
    ]);
  });

  it("omits customize/deploy only for the reviewed default-schema condition", () => {
    const fanout = web06SchemaChangeFanout({
      nextSchema: "jyut6ping3",
      deployPreferences,
      liveOptions,
      applyDeployPreferences: false,
    });
    expect(fanout).toHaveLength(13);
    expect(fanout[0]?.action.name).toBe("selectSchema");
    expect(fanout.slice(1).every(item => item.action.name === "setOption")).toBe(true);
  });
});
