import { useId, type ReactNode } from "react";
export default function Field({
  label,
  value,
  onChange,
  options,
  secret = false,
  multiline = false,
  disabled = false,
  hint,
}: {
  label: string;
  value: string | boolean;
  onChange: (value: string | boolean) => void;
  options?: Record<string, string>;
  secret?: boolean;
  multiline?: boolean;
  disabled?: boolean;
  hint?: ReactNode;
}) {
  const id = useId();
  if (typeof value === "boolean")
    return (
      <label className="check-field">
        <input
          id={id}
          type="checkbox"
          checked={value}
          disabled={disabled}
          onChange={(event) => onChange(event.target.checked)}
        />
        <span>{label}</span>
      </label>
    );
  return (
    <label className="field" htmlFor={id}>
      <span>{label}</span>
      {options ? (
        <select
          id={id}
          value={value}
          disabled={disabled}
          onChange={(event) => onChange(event.target.value)}
        >
          {Object.entries(options).map(([key, text]) => (
            <option key={key} value={key}>
              {text}
            </option>
          ))}
        </select>
      ) : multiline && !secret ? (
        <textarea
          id={id}
          value={value}
          disabled={disabled}
          onChange={(event) => onChange(event.target.value)}
        />
      ) : (
        <input
          id={id}
          type={secret ? "password" : "text"}
          value={value}
          disabled={disabled}
          autoComplete="off"
          spellCheck={false}
          onChange={(event) => onChange(event.target.value)}
        />
      )}{" "}
      {hint && <small>{hint}</small>}
    </label>
  );
}
