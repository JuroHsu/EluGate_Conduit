import { useEffect, useState } from "react";
import { api, errMsg, type EnvInfo, type Prefs } from "../api";
import { LANGUAGES, useI18n } from "../i18n";

export function PrefsPage() {
  const { t } = useI18n();
  const [env, setEnv] = useState<EnvInfo | null>(null);
  const [prefs, setPrefs] = useState<Prefs>({});
  const [saved, setSaved] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const reload = () => {
    api.getEnv().then(setEnv).catch((e) => setError(errMsg(e)));
    api.getPrefs().then(setPrefs);
  };
  useEffect(reload, []);

  const persist = async (next: Prefs) => {
    setError(null);
    try {
      await api.setPrefs({
        sshPath: next.sshPath?.trim() || null,
        sshConfigPath: next.sshConfigPath?.trim() || null,
        language: next.language || null,
      });
      setPrefs(next);
      return true;
    } catch (e) {
      setError(errMsg(e));
      return false;
    }
  };

  const save = async () => {
    if (await persist(prefs)) {
      setSaved(true);
      setTimeout(() => setSaved(false), 1500);
      reload();
    }
  };

  const toggleAutostart = async (on: boolean) => {
    try {
      await api.setAutostart(on);
      reload();
    } catch (e) {
      setError(errMsg(e));
    }
  };

  return (
    <div className="page">
      <header className="page-head">
        <div>
          <h1>{t("prefs.title")}</h1>
        </div>
      </header>

      <section className="card">
        <h2>{t("prefs.language")}</h2>
        {/* 語言變更立即套用到所有視窗與系統匣 */}
        <select
          className="lang-select"
          value={prefs.language ?? ""}
          aria-label={t("prefs.language")}
          onChange={(e) => persist({ ...prefs, language: e.target.value || null })}
        >
          <option value="">{t("prefs.languageAuto")}</option>
          {LANGUAGES.map(([code, name]) => (
            <option key={code} value={code}>
              {name}
            </option>
          ))}
        </select>
      </section>

      <section className="card">
        <h2>{t("prefs.startup")}</h2>
        <label className="check">
          <input type="checkbox" checked={env?.autostart ?? false} onChange={(e) => toggleAutostart(e.target.checked)} />
          {t("prefs.autostart", { place: t(env?.platform === "macos" ? "prefs.placeMenubar" : "prefs.placeTray") })}
        </label>
        <p className="muted small">{t("prefs.autostartHint")}</p>
      </section>

      <section className="card">
        <h2>OpenSSH</h2>
        {env?.ssh.error ? (
          <div className="notice error">{env.ssh.error}</div>
        ) : (
          <table className="kv">
            <tbody>
              <tr>
                <th>{t("prefs.sshInUse")}</th>
                <td className="mono">{env?.ssh.path}</td>
              </tr>
              <tr>
                <th>{t("prefs.version")}</th>
                <td className="mono">{env?.ssh.version}</td>
              </tr>
            </tbody>
          </table>
        )}
        <label className="field">
          <span className="field-label">{t("prefs.sshPath")}</span>
          <input
            className="mono"
            value={prefs.sshPath ?? ""}
            placeholder={t("prefs.sshPathPlaceholder")}
            onChange={(e) => setPrefs({ ...prefs, sshPath: e.target.value })}
            spellCheck={false}
          />
        </label>
        <label className="field">
          <span className="field-label">{t("prefs.defaultConfig")}</span>
          <input
            className="mono"
            value={prefs.sshConfigPath ?? ""}
            placeholder={env?.defaultConfig}
            onChange={(e) => setPrefs({ ...prefs, sshConfigPath: e.target.value })}
            spellCheck={false}
          />
        </label>
        <div className="row gap center">
          <button className="primary" onClick={save}>
            {t("common.save")}
          </button>
          {saved && <span className="tag ok">{t("common.saved")}</span>}
        </div>
        {error && <div className="notice error">{error}</div>}
      </section>

      <section className="card">
        <h2>{t("prefs.about")}</h2>
        <table className="kv">
          <tbody>
            <tr>
              <th>{t("prefs.version")}</th>
              <td>{env?.version}</td>
            </tr>
            <tr>
              <th>{t("prefs.dataDir")}</th>
              <td className="mono">{env?.dataDir}</td>
            </tr>
            <tr>
              <th>{t("prefs.logDir")}</th>
              <td className="mono">{env?.logDir}</td>
            </tr>
          </tbody>
        </table>
        <div className="row gap">
          <button className="danger" onClick={() => api.quit()}>
            {t("prefs.quit")}
          </button>
        </div>
      </section>
    </div>
  );
}
