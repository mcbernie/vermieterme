import { describe, expect, it } from "vitest";
import { createDirtyKeys } from "@/features/billing/use-billing-save";

/** A load keeps every key not confirmed stored when it started (#64). */
describe("createDirtyKeys with a load start", () => {
  it("keeps a key changed before a load started unsaved for it, even when saved meanwhile (#64)", () => {
    const dirty = createDirtyKeys();
    dirty.mark("cost:c1");
    const loadStart = dirty.settled();
    dirty.settle(dirty.revision());
    expect([...dirty.keys()]).toEqual([]);
    expect([...dirty.keys(loadStart)]).toEqual(["cost:c1"]);
  });

  it("lets a load take a key saved before it started from the server (#64)", () => {
    const dirty = createDirtyKeys();
    dirty.mark("cost:c1");
    dirty.settle(dirty.revision());
    const loadStart = dirty.settled();
    expect([...dirty.keys(loadStart)]).toEqual([]);
  });

  it("keeps a key whose save failed during a load unsaved for it (#64)", () => {
    const dirty = createDirtyKeys();
    dirty.mark("cost:c1");
    dirty.settle(dirty.revision()); // an earlier save succeeded
    dirty.mark("cost:c2");
    const failedRun = dirty.revision(); // the save of this revision starts ...
    const loadStart = dirty.settled(); // ... a load starts meanwhile ...
    expect(failedRun).toBeGreaterThan(loadStart);
    // ... and the save fails: no `settle(failedRun)` follows.
    expect([...dirty.keys(loadStart)]).toEqual(["cost:c2"]);
    expect([...dirty.keys()]).toEqual(["cost:c2"]);
  });
});
