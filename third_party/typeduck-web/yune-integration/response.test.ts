import { describe, expect, it } from "vitest";

import { translateResponse } from "./response.js";

describe("translateResponse", () => {
  it("maps candidate text, candidate comment, and highlighted index from the runtime shape", () => {
    const result = translateResponse({
      handled: true,
      commits: [],
      context: {
        input: "zyu",
        preedit: "zyu",
        caret: 2,
        highlighted: 1,
        page_size: 5,
        page_no: 2,
        is_last_page: false,
        select_keys: "12345",
        select_labels: ["1", "2"],
        candidates: [
          { text: "豬", comment: "zyu1" },
          { text: "主", comment: "zyu2" },
        ],
      },
      status: null,
    });

    expect(result).toMatchObject({
      highlightedIndex: 1,
      candidates: [
        { label: "1", text: "豬", comment: "zyu1" },
        { label: "2", text: "主", comment: "zyu2" },
      ],
    });
  });
});
