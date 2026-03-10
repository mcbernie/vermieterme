import { apiHandler, requireAuth, jsonOk, ApiError } from "@/lib/api-utils";
import {
  parseBundesbankResponse,
  BASE_YEAR_CODES,
  type BundesbankResponse,
} from "@/lib/vpi";

/**
 * Fetches VPI data from the Deutsche Bundesbank SDMX API.
 * Series: BBDP1 / M.DE.N.VPI.C.A00000.{baseCode}.A
 * This is the Verbraucherpreisindex (Gesamtindex) for Germany.
 * No authentication required.
 */
async function fetchVpiFromBundesbank(baseYear: number) {
  const baseCode = BASE_YEAR_CODES[baseYear];
  if (!baseCode) {
    throw new ApiError(
      `Basisjahr ${baseYear} wird nicht unterstützt. Verfügbar: ${Object.keys(BASE_YEAR_CODES).join(", ")}`,
      400
    );
  }

  const currentYear = new Date().getFullYear();
  const currentMonth = String(new Date().getMonth() + 1).padStart(2, "0");
  const startYear = Math.max(baseYear, currentYear - 5);

  const url = `https://api.statistiken.bundesbank.de/rest/data/BBDP1/M.DE.N.VPI.C.A00000.${baseCode}.A?detail=dataonly&startPeriod=${startYear}-01&endPeriod=${currentYear}-${currentMonth}`;

  const response = await fetch(url, {
    headers: { Accept: "application/json" },
    signal: AbortSignal.timeout(15000),
  });

  if (!response.ok) {
    throw new ApiError(
      `Bundesbank API Fehler: ${response.status} ${response.statusText}`,
      502
    );
  }

  const json: BundesbankResponse = await response.json();
  return parseBundesbankResponse(json);
}

export function POST(request: Request) {
  return apiHandler(async () => {
    await requireAuth();
    const body = await request.json();
    const { baseYear } = body;

    if (!baseYear || typeof baseYear !== "number") {
      throw new ApiError("Basisjahr ist erforderlich", 400);
    }

    try {
      const data = await fetchVpiFromBundesbank(baseYear);

      if (data.length === 0) {
        throw new ApiError(
          "Keine VPI-Daten gefunden. Bitte VPI-Werte manuell eintragen.",
          404
        );
      }

      return jsonOk({ entries: data, baseYear });
    } catch (error) {
      if (error instanceof ApiError) throw error;
      throw new ApiError(
        "Fehler beim Abrufen der VPI-Daten. Bitte VPI-Werte manuell eintragen.",
        502
      );
    }
  });
}
