// Form controls of the design system.

import { memo, useEffect, useId, useState, type ReactNode } from "react";
import type { Range } from "../types/models";
import { clamp, normalizeHex, snapToStep } from "../services/spec";

export function Toggle(props: { checked: boolean; onChange: (v: boolean) => void; disabled?: boolean; label?: string }) {
  return (
    <button
      type="button"
      role="switch"
      className="toggle"
      aria-checked={props.checked}
      aria-label={props.label}
      disabled={props.disabled}
      onClick={() => props.onChange(!props.checked)}
    />
  );
}

export function ToggleRow(props: {
  title: string;
  hint?: string;
  checked: boolean;
  onChange: (v: boolean) => void;
  disabled?: boolean;
}) {
  return (
    <div className="toggle-row">
      <div className="text">
        <span className="title">{props.title}</span>
        {props.hint && <span className="field-hint">{props.hint}</span>}
      </div>
      <Toggle checked={props.checked} onChange={props.onChange} disabled={props.disabled} label={props.title} />
    </div>
  );
}

interface SliderProps {
  label: string;
  value: number;
  range: Range;
  onChange: (v: number) => void;
  unit?: string;
  disabled?: boolean;
  /** Displayed value multiplier (e.g. 100 for percentages). */
  displayFactor?: number;
  inline?: boolean;
}

/** Range input + numeric field. Memoized: only re-renders when its value changes. */
export const SliderField = memo(function SliderField({
  label,
  value,
  range,
  onChange,
  unit,
  disabled,
  displayFactor = 1,
  inline,
}: SliderProps) {
  const id = useId();
  const shown = Number((value * displayFactor).toFixed(2));
  const [text, setText] = useState(String(shown));
  useEffect(() => setText(String(shown)), [shown]);
  const fill = ((value - range.min) / (range.max - range.min)) * 100;

  const commitText = () => {
    const parsed = Number(text.replace(",", "."));
    if (!Number.isFinite(parsed)) {
      setText(String(shown));
      return;
    }
    const v = clamp(snapToStep(parsed / displayFactor, range.step, range.min), range.min, range.max);
    setText(String(Number((v * displayFactor).toFixed(2))));
    if (v !== value) onChange(v);
  };

  return (
    <div className={`slider-field${inline ? " inline" : ""}`}>
      <label className="field-label" htmlFor={id}>
        {label}
      </label>
      <input
        id={id}
        className="range"
        type="range"
        min={range.min}
        max={range.max}
        step={range.step}
        value={value}
        disabled={disabled}
        style={{ ["--fill" as string]: `${clamp(fill, 0, 100)}%` }}
        onChange={(e) => onChange(snapToStep(Number(e.target.value), range.step, range.min))}
      />
      <div className="number-input">
        <input
          inputMode="decimal"
          value={text}
          disabled={disabled}
          aria-label={label}
          onChange={(e) => setText(e.target.value)}
          onBlur={commitText}
          onKeyDown={(e) => {
            if (e.key === "Enter") (e.target as HTMLInputElement).blur();
            if (e.key === "ArrowUp" || e.key === "ArrowDown") {
              e.preventDefault();
              const dir = e.key === "ArrowUp" ? 1 : -1;
              const step = range.step * (e.shiftKey ? 10 : 1);
              onChange(clamp(snapToStep(value + dir * step, range.step, range.min), range.min, range.max));
            }
          }}
        />
        {unit && <span className="unit">{unit}</span>}
      </div>
    </div>
  );
});

const SWATCHES = ["#00FF66", "#00FFFF", "#FFFF00", "#FF00FF", "#FF3B30", "#FFFFFF", "#8B5CF6", "#000000"];

export const ColorField = memo(function ColorField(props: { label: string; value: string; onChange: (v: string) => void; disabled?: boolean }) {
  const [text, setText] = useState(props.value.toUpperCase());
  const [invalid, setInvalid] = useState(false);
  useEffect(() => {
    setText(props.value.toUpperCase());
    setInvalid(false);
  }, [props.value]);

  const commit = (raw: string) => {
    const hex = normalizeHex(raw);
    if (hex) {
      setInvalid(false);
      if (hex !== props.value.toUpperCase()) props.onChange(hex);
    } else {
      setInvalid(true);
    }
  };

  return (
    <div className="field">
      <span className="field-label">{props.label}</span>
      <div className="color-field">
        <label className="color-swatch" title={props.label}>
          <span style={{ background: props.value }} />
          <input
            type="color"
            value={props.value.toLowerCase()}
            disabled={props.disabled}
            onChange={(e) => props.onChange(e.target.value.toUpperCase())}
          />
        </label>
        <input
          className={`input color-hex${invalid ? " invalid" : ""}`}
          value={text}
          maxLength={7}
          spellCheck={false}
          disabled={props.disabled}
          aria-label={props.label}
          onChange={(e) => {
            setText(e.target.value);
            if (normalizeHex(e.target.value)?.length === 7 && e.target.value.replace("#", "").length === 6) commit(e.target.value);
          }}
          onBlur={() => commit(text)}
          onKeyDown={(e) => e.key === "Enter" && commit(text)}
        />
        <div className="swatches">
          {SWATCHES.map((c) => (
            <button
              key={c}
              type="button"
              aria-label={c}
              aria-pressed={props.value.toUpperCase() === c}
              disabled={props.disabled}
              style={{ background: c }}
              onClick={() => props.onChange(c)}
            />
          ))}
        </div>
      </div>
    </div>
  );
});

export function Segmented<T extends string | number>(props: {
  value: T;
  options: { value: T; label: ReactNode; title?: string }[];
  onChange: (v: T) => void;
  block?: boolean;
  label?: string;
}) {
  return (
    <div className={`segmented${props.block ? " block" : ""}`} role="group" aria-label={props.label}>
      {props.options.map((o) => (
        <button
          key={String(o.value)}
          type="button"
          title={o.title}
          aria-pressed={o.value === props.value}
          onClick={() => props.onChange(o.value)}
        >
          {o.label}
        </button>
      ))}
    </div>
  );
}

export function Field(props: { label: string; hint?: string; children: ReactNode }) {
  return (
    <label className="field">
      <span className="field-label">{props.label}</span>
      {props.children}
      {props.hint && <span className="field-hint">{props.hint}</span>}
    </label>
  );
}

export function Kbd({ parts }: { parts: string[] }) {
  return (
    <span className="kbd">
      {parts.map((p, i) => (
        <kbd key={i}>{p}</kbd>
      ))}
    </span>
  );
}
