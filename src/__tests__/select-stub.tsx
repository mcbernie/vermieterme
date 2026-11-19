/**
 * Test double for the library `Select`: its popover closes again at once inside a `Modal` under
 * happy-dom (focus trap), so dialog tests replace it with plain buttons. Use it via
 * `vi.mock("@zweiund40gmbh/ui", async (orig) => ({ ...(await orig()), Select: SelectStub }))`.
 */
export function SelectStub<V extends string>({
  options,
  value,
  onChange,
  "aria-label": ariaLabel,
}: {
  options: { value: V; label: string }[];
  value: V | null | undefined;
  onChange: (value: V) => void;
  "aria-label"?: string;
}) {
  return (
    <div role="listbox" aria-label={ariaLabel} data-value={value ?? ""}>
      {options.map((o) => (
        <button key={o.value} type="button" role="option" aria-selected={o.value === value} onClick={() => onChange(o.value)}>
          {o.label}
        </button>
      ))}
    </div>
  );
}
