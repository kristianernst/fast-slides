"use client";

import { Moon, Sun } from "@solar-icons/react";

type SlideTokenValues = {
  slideBg: string;
  slideBorder: string;
  slideRadius: string;
  slidePadding: string;
  slideLayoutGap: string;
  slideCardBg: string;
  slideCardBorder: string;
  slideCardRadius: string;
  slideCardPadding: string;
  slideFontFamily: string;
  slideHeadingFont: string;
  slideCodeFont: string;
  slideMetaFont: string;
  slideMetaSize: string;
  slideFg: string;
  slideH1Color: string;
  slideH2Color: string;
  slideH3Color: string;
  slideBodyColor: string;
  slideMetaColor: string;
  slideAccent: string;
  slideLinkColor: string;
  slideCodeBg: string;
  slidePalette1: string;
  slidePalette2: string;
  slidePalette3: string;
  slidePalette4: string;
  slidePalette5: string;
};

type SlideTokenKey = keyof SlideTokenValues;

type SettingsOverlayProps = {
  open: boolean;
  busy: boolean;
  theme: "dark" | "light";
  onThemeChange: (theme: "dark" | "light") => void;
  mermaidThemeName: string;
  mermaidThemeOptions: readonly string[];
  mermaidThemeLabels: Record<string, string>;
  onMermaidThemeChange: (value: string) => void;
  syntaxThemeName: string;
  syntaxThemeOptionsByMode: {
    dark: readonly string[];
    light: readonly string[];
  };
  syntaxThemeLabels: Record<string, string>;
  onSyntaxThemeChange: (value: string) => void;
  selectedProject: boolean;
  slideTokens: SlideTokenValues;
  onUpdateToken: (key: SlideTokenKey, value: string) => void;
  onSaveCss: () => void;
  fontOptions: readonly string[];
  monoFontOptions: readonly string[];
  fontLabel: (value: string) => string;
  hexForInput: (value: string) => string;
  isHexColor: (value: string) => boolean;
};

const COLOR_FIELDS: Array<{ key: SlideTokenKey; label: string }> = [
  { key: "slideBg", label: "Background" },
  { key: "slideFg", label: "Text" },
  { key: "slideH1Color", label: "H1" },
  { key: "slideH2Color", label: "H2" },
  { key: "slideBodyColor", label: "Body" },
  { key: "slideMetaColor", label: "Meta text" },
  { key: "slideAccent", label: "Accent" },
  { key: "slideBorder", label: "Border" },
  { key: "slideLinkColor", label: "Link" },
  { key: "slideCodeBg", label: "Code background" },
];

const PALETTE_KEYS: SlideTokenKey[] = [
  "slidePalette1",
  "slidePalette2",
  "slidePalette3",
  "slidePalette4",
  "slidePalette5",
];

const LAYOUT_FIELDS: Array<{ key: SlideTokenKey; label: string }> = [
  { key: "slideRadius", label: "Radius" },
  { key: "slidePadding", label: "Padding" },
  { key: "slideLayoutGap", label: "Content gap" },
];

const COMPONENT_FIELDS: Array<{ key: SlideTokenKey; label: string }> = [
  { key: "slideCardBg", label: "Card background" },
  { key: "slideCardBorder", label: "Card border" },
  { key: "slideCardRadius", label: "Card radius" },
  { key: "slideCardPadding", label: "Card padding" },
];

export function SettingsOverlay({
  open,
  busy,
  theme,
  onThemeChange,
  mermaidThemeName,
  mermaidThemeOptions,
  mermaidThemeLabels,
  onMermaidThemeChange,
  syntaxThemeName,
  syntaxThemeOptionsByMode,
  syntaxThemeLabels,
  onSyntaxThemeChange,
  selectedProject,
  slideTokens,
  onUpdateToken,
  onSaveCss,
  fontOptions,
  monoFontOptions,
  fontLabel,
  hexForInput,
  isHexColor,
}: SettingsOverlayProps) {
  if (!open) {
    return null;
  }

  return (
    <div className="settings-overlay">
      <div className="settings-dialog">
        <header className="settings-header">
          <h2>Settings</h2>
        </header>
        <div className="settings-body">
          <div className="settings-section">
            <span className="settings-label">Theme</span>
            <div className="theme-toggle">
              <button
                type="button"
                className={theme === "dark" ? "active" : ""}
                onClick={() => onThemeChange("dark")}
              >
                <Moon size={14} weight="Linear" /> Dark
              </button>
              <button
                type="button"
                className={theme === "light" ? "active" : ""}
                onClick={() => onThemeChange("light")}
              >
                <Sun size={14} weight="Linear" /> Light
              </button>
            </div>
          </div>

          <div className="settings-section">
            <span className="settings-label">Mermaid</span>
            <div className="token-grid">
              <label className="token-row">
                <span className="token-name">Theme</span>
                <select
                  className="token-select"
                  value={mermaidThemeName}
                  onChange={(event) => onMermaidThemeChange(event.target.value)}
                >
                  {mermaidThemeOptions.map((option) => (
                    <option key={option} value={option}>
                      {mermaidThemeLabels[option] || option}
                    </option>
                  ))}
                </select>
              </label>
            </div>
          </div>

          <div className="settings-section">
            <span className="settings-label">Code blocks</span>
            <div className="token-grid">
              <label className="token-row">
                <span className="token-name">Editor theme</span>
                <select
                  className="token-select"
                  value={syntaxThemeName}
                  onChange={(event) => onSyntaxThemeChange(event.target.value)}
                >
                  <optgroup label="Dark">
                    {syntaxThemeOptionsByMode.dark.map((option) => (
                      <option key={option} value={option}>
                        {syntaxThemeLabels[option] || option}
                      </option>
                    ))}
                  </optgroup>
                  <optgroup label="Light">
                    {syntaxThemeOptionsByMode.light.map((option) => (
                      <option key={option} value={option}>
                        {syntaxThemeLabels[option] || option}
                      </option>
                    ))}
                  </optgroup>
                </select>
              </label>
            </div>
          </div>

          {selectedProject && (
            <>
              <div className="settings-section">
                <span className="settings-label">Colors</span>
                <div className="token-grid">
                  {COLOR_FIELDS.map(({ key, label }) => (
                    <label key={key} className="token-row">
                      <span className="token-name">{label}</span>
                      <span className="color-field">
                        <input
                          type="color"
                          value={hexForInput(slideTokens[key])}
                          onChange={(event) => onUpdateToken(key, event.target.value)}
                        />
                        <input
                          type="text"
                          className="color-text"
                          value={slideTokens[key]}
                          onChange={(event) => onUpdateToken(key, event.target.value)}
                        />
                      </span>
                    </label>
                  ))}
                </div>
              </div>

              <div className="settings-section">
                <span className="settings-label">Palette</span>
                <div className="palette-row">
                  {PALETTE_KEYS.map((key) => (
                    <label key={key} className="palette-swatch">
                      <input
                        type="color"
                        value={hexForInput(slideTokens[key])}
                        onChange={(event) => onUpdateToken(key, event.target.value)}
                      />
                      <span
                        className="palette-preview"
                        style={{
                          background: isHexColor(slideTokens[key]) ? slideTokens[key] : "#888",
                        }}
                      />
                    </label>
                  ))}
                </div>
              </div>

              <div className="settings-section">
                <span className="settings-label">Typography</span>
                <div className="token-grid">
                  <label className="token-row">
                    <span className="token-name">Font</span>
                    <select
                      className="token-select"
                      value={slideTokens.slideFontFamily}
                      onChange={(event) => onUpdateToken("slideFontFamily", event.target.value)}
                    >
                      {fontOptions.map((option) => (
                        <option key={option} value={option}>
                          {fontLabel(option)}
                        </option>
                      ))}
                    </select>
                  </label>
                  <label className="token-row">
                    <span className="token-name">Heading font</span>
                    <select
                      className="token-select"
                      value={slideTokens.slideHeadingFont}
                      onChange={(event) => onUpdateToken("slideHeadingFont", event.target.value)}
                    >
                      <option value="var(--slide-font-family)">Same as body</option>
                      {fontOptions.map((option) => (
                        <option key={option} value={option}>
                          {fontLabel(option)}
                        </option>
                      ))}
                    </select>
                  </label>
                  <label className="token-row">
                    <span className="token-name">Code font</span>
                    <select
                      className="token-select"
                      value={slideTokens.slideCodeFont}
                      onChange={(event) => onUpdateToken("slideCodeFont", event.target.value)}
                    >
                      {monoFontOptions.map((option) => (
                        <option key={option} value={option}>
                          {fontLabel(option)}
                        </option>
                      ))}
                    </select>
                  </label>
                  <label className="token-row">
                    <span className="token-name">Meta font</span>
                    <select
                      className="token-select"
                      value={slideTokens.slideMetaFont}
                      onChange={(event) => onUpdateToken("slideMetaFont", event.target.value)}
                    >
                      <option value="var(--slide-code-font)">Same as code</option>
                      {monoFontOptions.map((option) => (
                        <option key={option} value={option}>
                          {fontLabel(option)}
                        </option>
                      ))}
                    </select>
                  </label>
                  <label className="token-row">
                    <span className="token-name">Meta size</span>
                    <input
                      type="text"
                      className="token-input"
                      value={slideTokens.slideMetaSize}
                      onChange={(event) => onUpdateToken("slideMetaSize", event.target.value)}
                    />
                  </label>
                </div>
              </div>

              <div className="settings-section">
                <span className="settings-label">Layout</span>
                <div className="token-grid">
                  {LAYOUT_FIELDS.map(({ key, label }) => (
                    <label key={key} className="token-row">
                      <span className="token-name">{label}</span>
                      <input
                        type="text"
                        className="token-input"
                        value={slideTokens[key]}
                        onChange={(event) => onUpdateToken(key, event.target.value)}
                      />
                    </label>
                  ))}
                </div>
              </div>

              <div className="settings-section">
                <span className="settings-label">Components</span>
                <div className="token-grid">
                  {COMPONENT_FIELDS.map(({ key, label }) => {
                    const isColor = key.endsWith("Bg") || key.endsWith("Border");
                    return (
                      <label key={key} className="token-row">
                        <span className="token-name">{label}</span>
                        {isColor ? (
                          <span className="color-field">
                            <input
                              type="color"
                              value={hexForInput(slideTokens[key])}
                              onChange={(event) => onUpdateToken(key, event.target.value)}
                            />
                            <input
                              type="text"
                              className="color-text"
                              value={slideTokens[key]}
                              onChange={(event) => onUpdateToken(key, event.target.value)}
                            />
                          </span>
                        ) : (
                          <input
                            type="text"
                            className="token-input"
                            value={slideTokens[key]}
                            onChange={(event) => onUpdateToken(key, event.target.value)}
                          />
                        )}
                      </label>
                    );
                  })}
                </div>
              </div>

              <button
                type="button"
                className="btn btn-primary settings-save-btn"
                onClick={onSaveCss}
                disabled={busy}
              >
                Save to slides.css
              </button>
            </>
          )}
        </div>
      </div>
    </div>
  );
}
