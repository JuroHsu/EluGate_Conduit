import { useEffect, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { api, type Prompt } from "../api";
import { useI18n } from "../i18n";

/** ssh 透過 askpass 送來的驗證提示；回答只送回 ssh，不保存 */
export function AuthDialog({ id }: { id: string }) {
  const { t } = useI18n();
  const [prompt, setPrompt] = useState<Prompt | null | undefined>(undefined);
  const [value, setValue] = useState("");

  useEffect(() => {
    api.getPrompt(id).then(setPrompt);
  }, [id]);

  const answer = async (v: string | null) => {
    await api.answerPrompt(id, v);
    getCurrentWindow().destroy();
  };

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => e.key === "Escape" && answer(null);
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  });

  if (prompt === undefined) return null;
  if (prompt === null)
    return (
      <div className="auth">
        <p className="muted">{t("auth.ended")}</p>
        <button onClick={() => getCurrentWindow().destroy()}>{t("common.close")}</button>
      </div>
    );

  if (prompt.kind === "hostkey" || prompt.kind === "confirm") {
    const isHostKey = prompt.kind === "hostkey";
    return (
      <div className="auth">
        <h1>{t(isHostKey ? "auth.hostKeyTitle" : "auth.confirmTitle")}</h1>
        <p className="muted small">{t("auth.tunnel", { name: prompt.tunnelName })}</p>
        {isHostKey && <div className="notice warn">{t("auth.hostKeyNotice")}</div>}
        <pre className="prompt-text">{prompt.message}</pre>
        <div className="auth-actions">
          <button onClick={() => answer(isHostKey ? "no" : null)}>{t(isHostKey ? "auth.reject" : "common.cancel")}</button>
          <button className="primary" onClick={() => answer(isHostKey ? "yes" : "")} autoFocus>
            {t(isHostKey ? "auth.trust" : "auth.confirm")}
          </button>
        </div>
      </div>
    );
  }

  const title =
    prompt.kind === "passphrase" ? "auth.passphraseTitle" : prompt.kind === "password" ? "auth.passwordTitle" : "auth.otherTitle";
  return (
    <form
      className="auth"
      onSubmit={(e) => {
        e.preventDefault();
        answer(value);
      }}
    >
      <h1>{t(title)}</h1>
      <p className="muted small">{t("auth.tunnel", { name: prompt.tunnelName })}</p>
      <label className="field">
        <span className="field-label">{prompt.message}</span>
        <input
          type="password"
          autoFocus
          value={value}
          onChange={(e) => setValue(e.target.value)}
          autoComplete="off"
          spellCheck={false}
        />
      </label>
      <p className="muted small">{t("auth.privacy")}</p>
      <div className="auth-actions">
        <button type="button" onClick={() => answer(null)}>
          {t("common.cancel")}
        </button>
        <button type="submit" className="primary">
          {t("common.ok")}
        </button>
      </div>
    </form>
  );
}
