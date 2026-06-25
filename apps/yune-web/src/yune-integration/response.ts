import type { YuneWebResponse } from "@yune-ime/yune-web-runtime";
import type { RimeResult } from "../types";

export type { RimeResult };

/**
 * Upstream RimeResult shape from src/types.ts
 */
export function translateResponse(response: YuneWebResponse): RimeResult {
  if (!response.handled) {
    return { isComposing: false, success: false };
  }

  const committed = response.commits.length > 0 ? response.commits.join("") : undefined;

  if (response.context && response.context.preedit) {
    const preedit = response.context.preedit;
    const caretPos = response.context.caret ?? 0;
    const before = preedit.slice(0, caretPos);
    const active = preedit.slice(caretPos);
    const after = "";

    const candidates = response.context.candidates?.map((candidate, index) => ({
      label: response.context?.select_labels?.[index],
      text: candidate.text,
      comment: candidate.comment,
      source: candidate.source,
      quality: candidate.quality,
      preedit: candidate.preedit,
      aiConfidence: candidate.ai_confidence,
    })) ?? [];

    return {
      isComposing: true,
      inputBuffer: { before, active, after },
      page: response.context.page_no ?? 0,
      isLastPage: response.context.is_last_page ?? false,
      highlightedIndex: response.context.highlighted ?? 0,
      candidates,
      debug: response.context.debug,
      success: true,
      committed,
      status: response.status ?? undefined,
    };
  }

  return {
    isComposing: false,
    success: true,
    committed,
    status: response.status ?? undefined,
  };
}
