export type StartupSchema = "luna_pinyin" | "jyut6ping3_mobile";

export type StartupMode =
  | "real-worker-cold"
  | "real-worker-warm-reload"
  | "real-worker-warm-new-page"
  | "mock-worker-cold"
  | "mock-worker-warm";

export interface StartupScenario {
  id: string;
  schema: StartupSchema;
  mode: StartupMode;
  publicDemo: boolean;
  samples: number;
  inputs: string[];
}

export const trackAInputs = [
  "hao",
  "ni",
  "zhongguo",
  "ceshiyixiachangjushuruxingnengzenyang",
  "zhegeyinqingqishiyinggaizhichichaochangjuzishurucainengyong",
  "cszysmsrsd",
  "zybfshmsru",
] as const;

export const trackBInputs = [
  "hai",
  "ngo",
  "caksi",
  "sihaacoenggeoisyujapgecukdou",
  "taihaajyugwodaahoucoenggegeoizigosingnangwuidimjoeng",
] as const;

export const startupScenarios: StartupScenario[] = [
  { id: "tracked-luna-cold", schema: "luna_pinyin", mode: "real-worker-cold", publicDemo: false, samples: 10, inputs: [...trackAInputs] },
  { id: "tracked-luna-warm-reload", schema: "luna_pinyin", mode: "real-worker-warm-reload", publicDemo: false, samples: 20, inputs: [...trackAInputs] },
  { id: "tracked-luna-warm-new-page", schema: "luna_pinyin", mode: "real-worker-warm-new-page", publicDemo: false, samples: 20, inputs: [...trackAInputs] },
  { id: "tracked-jyut-cold", schema: "jyut6ping3_mobile", mode: "real-worker-cold", publicDemo: false, samples: 10, inputs: [...trackBInputs] },
  { id: "tracked-jyut-warm-reload", schema: "jyut6ping3_mobile", mode: "real-worker-warm-reload", publicDemo: false, samples: 20, inputs: [...trackBInputs] },
  { id: "tracked-jyut-warm-new-page", schema: "jyut6ping3_mobile", mode: "real-worker-warm-new-page", publicDemo: false, samples: 20, inputs: [...trackBInputs] },
  { id: "tracked-mock-cold", schema: "luna_pinyin", mode: "mock-worker-cold", publicDemo: false, samples: 10, inputs: ["hao"] },
  { id: "tracked-mock-warm", schema: "luna_pinyin", mode: "mock-worker-warm", publicDemo: false, samples: 20, inputs: ["hao"] },
  { id: "public-luna-cold", schema: "luna_pinyin", mode: "real-worker-cold", publicDemo: true, samples: 10, inputs: [...trackAInputs] },
  { id: "public-jyut-cold", schema: "jyut6ping3_mobile", mode: "real-worker-cold", publicDemo: true, samples: 10, inputs: [...trackBInputs] },
];

export function appSchemaId(schema: StartupSchema): "luna_pinyin" | "jyut6ping3" {
  return schema === "jyut6ping3_mobile" ? "jyut6ping3" : schema;
}

export function scenarioSamples(scenario: StartupScenario): number {
  const override = Number(process.env.M41_STARTUP_SAMPLES ?? "");
  if (Number.isFinite(override) && override > 0) {
    return Math.max(1, Math.floor(override));
  }
  if (process.env.M41_STARTUP_QUICK === "1") {
    return scenario.mode.includes("cold") ? 2 : 3;
  }
  return scenario.samples;
}
