import { useRef, type ComponentProps } from "react";
import { Icon } from "../Icon";
type Props = Omit<ComponentProps<"input">, "onChange"> & { onValueChange: (value: string) => void };
/** Shared search bar. The wrapper owns decoration; ref and input props reach the input. */
export function SearchField({ onValueChange, className = "", ref, value, disabled, ...props }: Props) {
  const input = useRef<HTMLInputElement | null>(null);
  return <div className={`muk-search-field ${className}`}>
    <Icon name="search" width="16" height="16" />
    <input {...props} ref={element => { input.current = element; if (typeof ref === "function") return ref(element); if (ref) ref.current = element; }}
      type="search" value={value} disabled={disabled} onChange={event => onValueChange(event.currentTarget.value)} />
    {String(value ?? "").length > 0 && <button type="button" disabled={disabled} aria-label="Clear search"
      onClick={() => { onValueChange(""); input.current?.focus(); }}><Icon name="close" width="14" height="14" /></button>}
  </div>;
}
export const SearchBar = SearchField;
