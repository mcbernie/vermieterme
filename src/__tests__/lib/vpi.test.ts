import { describe, it, expect } from "vitest";
import {
  parseBundesbankResponse,
  BASE_YEAR_CODES,
  type BundesbankResponse,
} from "@/lib/vpi";

describe("BASE_YEAR_CODES", () => {
  it("has codes for 2020, 2015, 2010", () => {
    expect(BASE_YEAR_CODES[2020]).toBe("I20");
    expect(BASE_YEAR_CODES[2015]).toBe("I15");
    expect(BASE_YEAR_CODES[2010]).toBe("I10");
  });

  it("returns undefined for unsupported year", () => {
    expect(BASE_YEAR_CODES[2000]).toBeUndefined();
  });
});

describe("parseBundesbankResponse", () => {
  it("parses a valid response", () => {
    const response: BundesbankResponse = {
      data: {
        structure: {
          dimensions: {
            observation: [
              {
                values: [
                  { id: "2024-01" },
                  { id: "2024-02" },
                  { id: "2024-03" },
                ],
              },
            ],
          },
        },
        dataSets: [
          {
            series: {
              "0:0:0:0:0:0:0:0": {
                observations: {
                  "0": ["117.6"],
                  "1": ["118.1"],
                  "2": ["118.6"],
                },
              },
            },
          },
        ],
      },
    };

    const result = parseBundesbankResponse(response);
    expect(result).toHaveLength(3);
    expect(result[0]).toEqual({ year: 2024, month: 1, value: 117.6 });
    expect(result[1]).toEqual({ year: 2024, month: 2, value: 118.1 });
    expect(result[2]).toEqual({ year: 2024, month: 3, value: 118.6 });
  });

  it("sorts by year and month", () => {
    const response: BundesbankResponse = {
      data: {
        structure: {
          dimensions: {
            observation: [
              {
                values: [
                  { id: "2025-01" },
                  { id: "2024-12" },
                  { id: "2024-11" },
                ],
              },
            ],
          },
        },
        dataSets: [
          {
            series: {
              "0:0:0:0:0:0:0:0": {
                observations: {
                  "0": ["120.3"],
                  "1": ["120.5"],
                  "2": ["119.9"],
                },
              },
            },
          },
        ],
      },
    };

    const result = parseBundesbankResponse(response);
    expect(result[0]).toEqual({ year: 2024, month: 11, value: 119.9 });
    expect(result[1]).toEqual({ year: 2024, month: 12, value: 120.5 });
    expect(result[2]).toEqual({ year: 2025, month: 1, value: 120.3 });
  });

  it("rounds values to 1 decimal", () => {
    const response: BundesbankResponse = {
      data: {
        structure: {
          dimensions: {
            observation: [{ values: [{ id: "2024-06" }] }],
          },
        },
        dataSets: [
          {
            series: {
              "0:0:0:0:0:0:0:0": {
                observations: { "0": ["119.35"] },
              },
            },
          },
        ],
      },
    };

    const result = parseBundesbankResponse(response);
    expect(result[0].value).toBe(119.4);
  });

  it("returns empty array for missing data", () => {
    const empty: BundesbankResponse = {
      data: {
        structure: {
          dimensions: { observation: [] },
        },
        dataSets: [],
      },
    };

    expect(parseBundesbankResponse(empty)).toEqual([]);
  });

  it("skips entries with invalid period format", () => {
    const response: BundesbankResponse = {
      data: {
        structure: {
          dimensions: {
            observation: [
              {
                values: [
                  { id: "2024-01" },
                  { id: "invalid" },
                  { id: "2024-03" },
                ],
              },
            ],
          },
        },
        dataSets: [
          {
            series: {
              "0:0:0:0:0:0:0:0": {
                observations: {
                  "0": ["117.6"],
                  "1": ["999.9"],
                  "2": ["118.6"],
                },
              },
            },
          },
        ],
      },
    };

    const result = parseBundesbankResponse(response);
    expect(result).toHaveLength(2);
    expect(result[0].month).toBe(1);
    expect(result[1].month).toBe(3);
  });

  it("skips NaN values", () => {
    const response: BundesbankResponse = {
      data: {
        structure: {
          dimensions: {
            observation: [{ values: [{ id: "2024-01" }] }],
          },
        },
        dataSets: [
          {
            series: {
              "0:0:0:0:0:0:0:0": {
                observations: { "0": ["not-a-number"] },
              },
            },
          },
        ],
      },
    };

    expect(parseBundesbankResponse(response)).toEqual([]);
  });
});
