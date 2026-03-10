export interface VpiDataPoint {
  year: number;
  month: number;
  value: number;
}

export interface BundesbankResponse {
  data: {
    structure: {
      dimensions: {
        observation: Array<{
          values: Array<{ id: string }>;
        }>;
      };
    };
    dataSets: Array<{
      series: Record<
        string,
        { observations: Record<string, [number | string]> }
      >;
    }>;
  };
}

// Base year to Bundesbank series code mapping
export const BASE_YEAR_CODES: Record<number, string> = {
  2020: "I20",
  2015: "I15",
  2010: "I10",
};

export function parseBundesbankResponse(
  json: BundesbankResponse
): VpiDataPoint[] {
  const results: VpiDataPoint[] = [];

  const timePeriods = json.data.structure.dimensions.observation[0]?.values;
  if (!timePeriods) return results;

  const dataSets = json.data.dataSets;
  if (!dataSets || dataSets.length === 0) return results;

  const series = dataSets[0].series;
  for (const seriesData of Object.values(series)) {
    for (const [idx, obs] of Object.entries(seriesData.observations)) {
      const period = timePeriods[parseInt(idx)];
      if (!period) continue;

      // Period format: "2024-01"
      const match = period.id.match(/^(\d{4})-(\d{2})$/);
      if (!match) continue;

      const raw = obs[0];
      const value = typeof raw === "string" ? parseFloat(raw) : raw;
      if (typeof value !== "number" || isNaN(value)) continue;

      results.push({
        year: parseInt(match[1]),
        month: parseInt(match[2]),
        value: Math.round(value * 10) / 10,
      });
    }
  }

  results.sort((a, b) => {
    if (a.year !== b.year) return a.year - b.year;
    return a.month - b.month;
  });

  return results;
}
