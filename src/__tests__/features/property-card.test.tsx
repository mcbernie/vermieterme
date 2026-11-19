import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { PropertyCard } from "@/features/properties/property-card";
import type { PropertyWithUnits } from "@/types";

const property = { id: "a", street: "Hauptstr. 1", zip: "12345", city: "Berlin", totalShares: 100, _count: { units: 0 } };
const detail: PropertyWithUnits = { ...property, units: [] };

/** Renders the card with spy handlers; returns them plus the render result. */
function setup(overrides: Partial<Parameters<typeof PropertyCard>[0]> = {}) {
  const handlers = {
    onToggle: vi.fn(),
    onEdit: vi.fn(),
    onDelete: vi.fn(),
    onAddUnit: vi.fn(),
    onEditUnit: vi.fn(),
    onDeleteUnit: vi.fn(),
  };
  const view = render(
    <PropertyCard
      property={property}
      expanded={false}
      detail={null}
      detailFailed={false}
      rentChanges={[]}
      {...handlers}
      {...overrides}
    />,
  );
  return { ...handlers, ...view };
}

describe("PropertyCard", () => {
  it("toggles when the header (text or free area) is clicked", () => {
    const h = setup();
    fireEvent.click(screen.getByText("Hauptstr. 1"));
    fireEvent.click(screen.getByText(/Anteile/));
    expect(h.onToggle).toHaveBeenCalledTimes(2);
  });

  it("toggles via the keyboard-reachable title button", () => {
    const h = setup();
    fireEvent.click(screen.getByRole("button", { name: /Hauptstr. 1/ }));
    expect(h.onToggle).toHaveBeenCalledTimes(1);
  });

  it("does not toggle when an action button is clicked", () => {
    const h = setup();
    fireEvent.click(screen.getByRole("button", { name: "Bearbeiten" }));
    fireEvent.click(screen.getByRole("button", { name: "Löschen" }));
    expect(h.onEdit).toHaveBeenCalledTimes(1);
    expect(h.onDelete).toHaveBeenCalledTimes(1);
    expect(h.onToggle).not.toHaveBeenCalled();
  });

  it("shows the loading state while the units load", () => {
    setup({ expanded: true });
    expect(screen.getByRole("status")).toBeInTheDocument();
    expect(screen.queryByText(/konnten nicht/)).toBeNull();
    expect(screen.queryByText("Wohnungen")).toBeNull();
  });

  it("shows an error instead of the loading state when loading failed", () => {
    setup({ expanded: true, detailFailed: true });
    expect(screen.getByText(/konnten nicht geladen werden/)).toBeInTheDocument();
    expect(screen.queryByRole("status")).toBeNull();
  });

  it("shows the unit section without loading or error once loaded", () => {
    setup({ expanded: true, detail });
    expect(screen.getByText("Wohnungen")).toBeInTheDocument();
    expect(screen.queryByRole("status")).toBeNull();
    expect(screen.queryByText(/konnten nicht/)).toBeNull();
  });

  it("warns that the list may be stale when a refresh failed", () => {
    setup({ expanded: true, detail, detailFailed: true });
    expect(screen.getByText(/konnten nicht aktualisiert werden/)).toBeInTheDocument();
    expect(screen.getByText("Wohnungen")).toBeInTheDocument();
  });
});
