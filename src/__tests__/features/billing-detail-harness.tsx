import { act, fireEvent, render, screen } from "@testing-library/react";
import { vi } from "vitest";
import { ConfirmProvider, UiProvider } from "@zweiund40gmbh/ui";
import { BillingDetailView } from "@/features/billing/billing-detail-view";
import { categories, detail, json } from "./billing-fixtures";

/** Answers a write request; `undefined` means the default answer. */
export type Handler = (url: string, init?: RequestInit) => Response | undefined;

/** Routes fetch: GETs answer with the fixture, `writes` may answer writes (default: ok). */
export function stub(writes: Handler = () => undefined, periodStatus = 200, period = detail, cats = categories) {
  const fetchMock = vi.fn(async (url: string, init?: RequestInit) => {
    const method = init?.method ?? "GET";
    if (method !== "GET") return writes(url, init) ?? json({});
    if (url === "/api/billing-periods/bp") return json(periodStatus === 200 ? period() : { error: "x" }, periodStatus);
    if (url === "/api/cost-categories") return json(cats);
    return json([]);
  });
  vi.stubGlobal("fetch", fetchMock);
  return fetchMock;
}

/** Renders the detail page of period `bp` and waits for the first load. */
export async function mount() {
  await act(async () => {
    render(
      <UiProvider locale="de-DE">
        <ConfirmProvider>
          <BillingDetailView id="bp" />
        </ConfirmProvider>
      </UiProvider>,
    );
  });
}

/** Write calls whose URL contains `part`. */
export const writesTo = (fetchMock: ReturnType<typeof stub>, part: string) =>
  fetchMock.mock.calls.filter((c) => c[1]?.method && c[1].method !== "GET" && c[0].includes(part));

/** Bodies of the amount saves (not the single-field updates) sent to `/costs`. */
export const amountSaves = (fetchMock: ReturnType<typeof stub>) =>
  writesTo(fetchMock, "/costs")
    .map((c) => JSON.parse(c[1]?.body as string))
    .filter((b) => "totalAmount" in b && !("reviewed" in b));

/** Types into a money field without leaving it (one input event). */
export async function typeOnly(label: string, text: string) {
  const input = screen.getByLabelText(label);
  await act(async () => fireEvent.focus(input));
  await act(async () => fireEvent.input(input, { target: { value: text } }));
  return input;
}

/** Clicks the save button. */
export const clickSave = () => act(async () => fireEvent.click(screen.getByRole("button", { name: "Speichern" })));

/** Types into a money field and leaves it. */
export async function typeMoney(label: string, text: string) {
  const input = screen.getByLabelText(label);
  await act(async () => fireEvent.focus(input));
  await act(async () => fireEvent.change(input, { target: { value: text } }));
  await act(async () => fireEvent.blur(input));
}
