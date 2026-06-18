import type { TypeDuckResponse } from "@yune-ime/typeduck-runtime";

/**
 * Upstream RimeResult shape from src/types.ts
 */
export interface RimeResult {
  isComposing: boolean;
  inputBuffer?: {
    before: string;
    active: string;
    after: string;
  };
  page?: number;
  isLastPage?: boolean;
  highlightedIndex?: number;
  candidates?: Array<{
    label?: string;
    text: string;
    comment?: string;
  }>;
  success: boolean;
  committed?: string;
}

export function translateResponse(response: TypeDuckResponse): RimeResult {
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
    }));

    return {
      isComposing: true,
      inputBuffer: { before, active, after },
      page: response.context.page_no ?? 0,
      isLastPage: response.context.is_last_page ?? false,
      highlightedIndex: response.context.highlighted ?? 0,
      candidates,
      success: true,
      committed,
    };
  }

  return {
    isComposing: false,
    success: true,
    committed,
  };
}
