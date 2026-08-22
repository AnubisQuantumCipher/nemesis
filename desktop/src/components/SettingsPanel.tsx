import { useEffect, useState } from "react";

import type { DesktopSettings } from "../lib/bridge";

interface SettingsPanelProps {
  settings: DesktopSettings;
  saving: boolean;
  onSave: (settings: DesktopSettings) => Promise<void>;
}

export function SettingsPanel({ settings, saving, onSave }: SettingsPanelProps) {
  const [draft, setDraft] = useState(settings);

  useEffect(() => {
    setDraft(settings);
  }, [settings]);

  return (
    <section className="settings-panel" aria-labelledby="settings-heading">
      <header className="panel-heading">
        <div>
          <span className="section-index">LOCAL / REVERSIBLE</span>
          <h2 id="settings-heading">Settings</h2>
        </div>
      </header>

      <form
        className="settings-form"
        onSubmit={(event) => {
          event.preventDefault();
          void onSave(draft);
        }}
      >
        <label>
          <span>Text scale</span>
          <select
            aria-label="Text scale"
            value={draft.textScale}
            onChange={(event) =>
              setDraft((current) => ({
                ...current,
                textScale: event.target.value as DesktopSettings["textScale"],
              }))
            }
          >
            <option value="standard">Standard — 12 px minimum</option>
            <option value="large">Large — 15 px minimum</option>
          </select>
        </label>

        <label className="setting-check">
          <input
            type="checkbox"
            checked={draft.reduceMotion}
            onChange={(event) =>
              setDraft((current) => ({ ...current, reduceMotion: event.target.checked }))
            }
          />
          <span>
            <strong>Reduce motion</strong>
            <small>Stops cockpit pulses and animated transitions.</small>
          </span>
        </label>


        <div className="settings-actions">
          <button type="submit" className="primary-action" disabled={saving}>
            {saving ? "Saving…" : "Save settings"}
          </button>
          <button type="button" className="text-button" onClick={() => setDraft(settings)} disabled={saving}>
            Revert unsaved changes
          </button>
        </div>
      </form>
    </section>
  );
}
